# 15 — v2 slices

**Implementation authority for v2.** Scope is in [`14-V2-SCOPE.md`](14-V2-SCOPE.md), and every v2 PR
belongs to exactly one slice.

**v2 reads eight formats, the format row is closed, and the gate is met.** DOCX, XLSX, PPTX, ODT,
ODS, ODP, RTF and EPUB, with **CSV an argued refusal rather than a reader**. Both questions that
stood between this repository and the gate were settled by the owner as decisions #16 and #17, and
the last technical question — whether the ZIP reader should verify the checksum it ignored — was
answered by measurement and shipped.

**v1 is not done**, and no slice here closes it. Note that v1's S7 and this document's S7 are
different slices in different versions.

| Slice | Theme | State |
| --- | --- | --- |
| **S0** | The scope document and this one | done |
| **S1** | The grounding contract for a page-less source | done — the answer is PDF-only, plus one finding |
| **S2** | DOCX, and the page-parent invariant S1 found | done |
| **S3** | XLSX: sheets and cells | done |
| **S4** | PPTX: a slide is a part | done |
| **S5** | ODT: the page break that is in the file | done |
| **S6** | ODS: the address the file never writes | done |
| **S7** | ODP: the page that was free | done |
| **S8** | RTF: the address with no part | done |
| **S9** | EPUB: the spine, and the page a publisher named | done |
| **S10** | CSV | **done — an argued refusal** |
| **S11** | Embedded assets, counted | done |
| **S12–S13** | The office readers get fuzzed, then mutated | done |
| **S14** | The checksum question, answered | done |
| **S15** | A wire-string cluster that had been wrong at birth | done |
| **S19** | The gate corpus grown from four documents to twelve | done |
| **S20** | Nine phantom grids the engine already rejected | done |
| **S22** | Why ten documents produce nothing | done |
| **S24** | Tagged tables | done |
| *(S9.1, S10.1, S10.2, S12.1, S13.1–S13.5, S14.1, S16–S18, S21, S23)* | Repairs to guards, prose and instruments | done — see the last section |

**The order was deliberate.** S1 is a decision with no parser, ahead of the reader whose output
depends on it — **so the first implementation cannot quietly acquire a page "for convenience" while
nobody has written down that it may not.**

---

## S0 — Scope and slice map

**Goal:** v2 exists as an ordered list of bounded changes **before** any office parser does, and the
no-synthesised-pages law is written down **before** the first format that can violate it.

**Why first:** because the cheapest way to make a DOCX quote "ground" is to print it to PDF and read
the page number off the result — **and that produces a citation that looks correct, validates against
the current schema, and is a measurement of a printer.**

**The remaining-formats row split six times, each against a measurement**, on a rule S0 set: it stays
one line until the cost of the next format in it has been *measured* rather than guessed. S4 measured
that a third OOXML format was one reader and one fixture pair. S5 measured that the first non-OOXML
format shared only the container. S6, S7, S8 and S9 each measured the next. **Writing a split while
somebody is already inside a reader is exactly when "while we're here, the next one is nearly free"
gets said.**

## S1 — The grounding contract for a page-less source

**Goal:** decide what *a DOCX quote grounds* means **before** a reader exists whose output depends on
the answer.

**The answer is that the schema stays PDF-only.** Revising it is a change to the **verifier's**
contract — an engine-only revision would produce artifacts the verifier does not speak while both
still called themselves the same thing. So that option was recorded as **blocked on a revision owned
elsewhere, not refused.**

**The finding: the page assumption is not where the scope document thought.** S0 wrote that under this
decision the gate would be met "at the representation level". **That is false.** The seal refuses a
node whose parent is not a declared page, on both construction paths — **so a page-less document
cannot become a representation at all.** Reaching the gate is upstream of grounding, in the record's
own page-parent invariant. S2 carries it, **and now knows it before writing a reader against an
invariant that would have rejected its output.**

**The architecture's precondition held, and it pointed at the wrong crate.** The projection never
learned what a page *is* — it reads no locator, derives no geometry, and addresses pages by id. Its
own parent check is a re-assertion of an invariant the seal already guarantees. The assumption lived
in the contract crate.

**One guard is about the dependency graph rather than the code:** no renderer in the lock file,
**because that refusal lapses as a transitive dependency before it lapses as a design decision.**

## S2 — DOCX

**The invariant changed the way S1 said it would have to.** The structure check now splits on the
**locator family** — the one thing a node cannot fake, since it reaches the page-less rules only by
carrying an address with no page in it:

| | Paginated address | Page-less address |
| --- | --- | --- |
| Parent | A declared page — unchanged | A part id |
| `pages` | As before | Must be **empty**, or the seal refuses "invented pagination" |
| Geometry | A measured box is checked against its page | A measured box is **refused** — there is no page to check it against |
| Integrity | The parent is looked up in a declared list | Part id ↔ part name is a **bijection**, so nothing needs a second list |

**That last row is the part worth arguing about.** A PDF gets its integrity from a declared-page
lookup; a DOCX has no such list, and inventing one would be a payload field for a format that
describes itself already. Instead every page-less node names its part in its own locator, and the
seal checks that one id means one name **in both directions.**

**The geometry row is what makes "no geometry" a fact about the artifact rather than a habit of the
reader:** the office crate could not emit a rectangle even if a later edit tried.

**The locator carries a part name and two document-order positions. No page, no box, no coordinates**,
with unknown fields denied so one cannot be added quietly. **If a field would have to be computed by
laying the document out, it does not belong here.**

**A new attributes variant rather than the PDF one with fields blanked**: a Word run has no character
codes, no font resource name and no font size this reader read, **and a zero font size would be three
claims the document never made.** The node *kind* is reused, because a Word run and a show-text run
are the same thing addressed differently, and the locator is what says which.

**One new dependency, and the asymmetry is the whole argument.** ZIP is read in-crate over the
compression library the graph already carried, because a ZIP crate drags a dozen transitives —
including a *compressor* — to save about 120 lines. XML is the opposite call and is deliberately **not**
hand-rolled: entities, namespaces, CDATA and encodings are exactly where a hand-rolled reader silently
gets *text* wrong, **and text is the evidence.**

**Measured, not feared:** the first version of the reader dropped `&amp;` silently, because the XML
library delivers an entity as its own event. The fixture carries an ampersand because of that, and
the reader now resolves the five predefined entities and **refuses every other name** rather than
letting one become an empty string in the evidence.

**Headers, footers, footnotes and comments are counted and declared**, because a reader that silently
returned the body would let a caller conclude a phrase is absent from a document that contains it.

**Detection is content-based**: a signature plus the main part's presence in the central directory, so
a renamed file reads and a mislabelled one is a named failure.

## S3 — XLSX

**The locator carries both the part and the sheet, because they answer different questions.** The part
is the package's name for the worksheet and is what the bijection checks; the sheet is the workbook's
name for it and is what a person citing a cell writes down. **The part name does not contain the sheet
name and never will.**

**The row is a number and the column is a string, and the asymmetry is the file's.** A cell reference
states the column as letters and the row as digits. Reading the digits is reading — the attribute's own
type is an integer. **Turning `B` into `2` is arithmetic on a bijective base-26 numeral, which is a
computation the file never performed and a value it never contains.** Concatenating the two reproduces
the reference exactly, so splitting it loses nothing.

**The part the shortcut would have skipped.** The workbook part contains **no part names at all** — a
sheet entry carries a name, an id and a relationship id, and only the relationships part says which
file a relationship id means. The shortcut of assuming sequential file names is wrong in *ordinary*
files: reordering sheets reorders the entries and leaves the file names alone, deleting a sheet leaves
a gap, and names are author-chosen. **Every one of those failures attaches the wrong sheet name to the
right cells — a locator that is confidently wrong, which is strictly worse than one that is absent.**

**The invariant did not have to change.** A workbook is one part per sheet, and S2's shape already
allowed it: the part check is a **bijection**, not a cardinality-of-one rule, and ordinals count per
parent. **S2 built the shape with one part and S3 is the first artifact to use it with more than one.**

**The node kind is reused, and here is the tradeoff.** For a workbook the cell *is* the atom, so the
question is whether "this text is a cell" names a fact no existing node carries. It does not — the
locator says sheet, row and column, which is cell-ness spelled out in the one place a consumer must
already look. A second kind would restate the locator and make every consumer handle two names for one
concept. **A new attributes variant carries the facts with no home:** what the stored value's type is,
and whether the text is a stored value, a cached formula result, or a formula's source because nothing
was cached.

**The table capability stays false on a format made of grids**, and that is not modesty: it means
*this run emitted table records*, and this slice emits cells. **A consumer reading true would go
looking for a table structure that is not there.**

**A formula is not a second authority.** No evaluator and no computed-value field. A cell's text is
the value the workbook **stored**, and a field says which — **so a formula's source can never be read
as a number the sheet displayed.**

**The coordinate declaration stays inert, deliberately.** Measured: nothing acts on the value for a
page-less artifact — the projection hard-codes it, the only branch on it lives downstream of a refusal,
and both SDKs never parse it. **A mode enum would have moved every PDF artifact's profile hash to
respell a value nothing reads.** What is pinned instead is the property that makes it honest: both
page-less profiles declare no measured ink boxes, and a test asserts zero measured geometry rows on a
real workbook artifact.

**Reviewing the slice against the standing rules found eight defects before it shipped, and fixing them
surfaced a ninth.** The pattern is worth keeping: **every one was a *silent* failure**, and three were
in code whose own comment described the hazard it had.

| | Defect | Why it was silent |
| --- | --- | --- |
| 1 | A self-closing shared-string entry took no slot in the table | Only the paired events were matched, so **every later index resolved to the next string** — a right address carrying another cell's text |
| 2 | Furigana concatenated into inline-string cells | The shared-string path stripped it; the inline path is the same content type and did not |
| 3 | A second value element appended instead of being refused | Two values became one concatenated number, which then resolved as a shared-string index |
| 4 | A type naming a child the cell did not have returned "no node" | Indistinguishable from an empty cell, so present characters vanished |
| 5 | Reference splitting repaired malformed references into valid ones | The integer parser accepts a sign and leading zeros, so **the locator spelled a different cell** |
| 6 | Two cells at one address both became nodes | A citation to that address would have had two answers |
| 7 | Errors reading the string table were swallowed | An oversized part re-surfaced as "the table has 0 entries", blaming a worksheet |
| 8 | A documented cross-check between a node's kind and its attributes did not exist | Prose since v0; a record could read as two different things depending on which field was trusted |
| 9 | A cell carrying two mutually exclusive value forms | Found while fixing 4: whichever the type named was read and the other dropped without a word |

**Numbers 1, 2 and 5 matter most, because each produced a sealed, byte-identical, error-free artifact
with the wrong text at the right address** — the failure this module's own header calls strictly worse
than no address at all.

## S4 — PPTX

**The temptation this format exists to test.** A DOCX has no page until a renderer decides one. A
spreadsheet's is a printer's. **A slide is neither** — it is discrete, addressable, listed in the
package, and a person counts them out loud. **This is the first v2 format where inventing a page would
not even feel like inventing one.**

It is still a **part**, and the two things that would have made it a page are refused by name:

| The tempting field | What it actually is |
| --- | --- |
| The slide size | A size the authoring tool wrote, which this engine measured nothing against |
| Position in the slide-id list | Display **order**, which changes when a deck is reordered, and which a consumer would read as a page number |

**So the locator carries no slide number at all.** A caller that wants deck position reads the
presentation part, **where it is a fact about the presentation rather than a claim baked into every
citation.** A test asserts the locator's exact field set, so a slide index cannot arrive later as a
fifth field.

**The shape component was going to be the shape's own id, and measurement changed it.** Across 18 real
decks — 329 slides, 3,335 shapes — the id is present every time and **unique only most of the time**:
12 slides from one generator reuse one, and the application opens them without complaint. **Addressing
by it would have given one address two answers on real files**, and refusing those files would have
rejected decks that open everywhere else. So the id moved to the attributes, where a non-unique label
is exactly what it is.

**What a reader that only saw top-level shapes would miss**, measured rather than assumed. Of 5,297
text elements across those decks: 4,670 in top-level shapes, 160 in nested groups, 287 in graphic
frames, 180 in fields. **A top-level-only reader would get 88.2% and say nothing about the rest.**
Descending into groups brings it to 91.2%, and **the remaining 8.8% is counted rather than dropped.**

**The field is the interesting refusal:** a slide-number field holds a **cached** number written at
save time that goes stale the moment the deck is reordered. **Reading it would put a number in the
evidence that is not on the screen and is shaped exactly like the page index this version refuses.**

**Alternate-content branches: one phrase, one node — and the counters still see the rest.** Such a
block appeared on 162 of 329 slides, and its branches all state the same content for consumers of
different capability. A descendant walk emits that phrase at two or three addresses — **the mirror
image of a silent drop.** Exactly one branch is read, deterministically, and the rest are counted when
they held text.

**But the counters still advance through the skipped branches**, and that is the half worth stating.
The address promises a position in the part's own document order, so a consumer checking it counts
elements in the file. **Counting only what was read would leave every address after a skipped branch
one short — a locator that is confidently wrong.** The same holds for a self-closing paragraph element
that two common libraries write for a blank line: **the address must not turn on how a deck was
serialized.**

**One rule moved to a module of its own.** Three formats now share the same answer to "what part does
this relationship id mean?", and **three of the nine defects the previous slice's review found were two
copies of one rule disagreeing.** Nothing in the moved code changed, and both existing suites passed
unaltered across the move — **which is what made it a move rather than a rewrite.**

**Reviewing against the standing rules found three more defects, all of the same class: a right address
pointing at the wrong thing.** None lost text; each made a locator disagree with what a consumer
counting elements would find. The sharpest is that every branch of an alternate-content block was being
read — **the code said reading both would emit the same text twice at two addresses, and then guarded
only one of the two ways that happens.**

## S5 — ODT

**The format that tested the law the hardest, and still did not move it.** An ODT's content part
**contains an actual page break element**, and the styles carry a paper width, so a page record would
have needed almost no arithmetic.

| What the file states | What it is |
| --- | --- |
| A soft page break | The position at which the *producing application's* layout broke the page, computed from its font stack and paper size and written down at save time |
| A page width and height | The paper the author chose, which this engine measured nothing against |

**It is read, recognised and discarded**, and `pages` stays empty.

**The atom is the paragraph, not the span.** A span is formatting, and addressing by it would make the
address depend on where the author changed a font.

**Detection asks a different question here**, and for ODF the bytes say so out loud: the **first**
central-directory entry must be a **stored** entry naming the format. That is the package
specification's own requirement, so it is a fact the file states rather than a heuristic.

**ODF's own whitespace rule is applied, and that is reading** — the format defines how repeated spaces
are encoded, so honouring it recovers what the document says rather than inventing it.

**Not read, and declared:** footnote and endnote bodies (a second stream of text); comments (reading one
would put a reviewer's remark in the record as the document's own); tracked-change deletions (**reading
what a revision removed would put text the document no longer states into the evidence**); and second
renditions inside a frame, because ODF frames hold *alternative* renditions of one object and reading
both emits one displayed phrase at two citable addresses.

**An adversarial review found what the author's own did not, and it inverted the rule.** Character data
was reaching a block through elements that are not text: an image's title and alt text, an embedded
object's base64 (**a kilobyte of encoded bytes spliced mid-sentence**), a heading's *generated* number,
a field's cached value, and furigana. **So the rule is inverted: character data reaches a block only
when every element between them is an allowlisted inline one.**

Three defects came with it, and each is a distinct failure mode: element names were matched by
**suffix**, so a conforming foreign element advanced the block counter and **shifted every later
address**; a mathematics annotation shares a local name with a comment, so an inline formula was
declared to the caller as an unread *reviewer's remark*; and a skip flag was one slot on the claim that
these regions "do not meaningfully nest" — **they do**, so two erased passages were declared as one,
**under-declaring, which is the direction the erasure rule exists to prevent.**

**Reviewing against the standing rules found four more before it shipped**, each in the same family: a
footnote concatenated into the sentence citing it, producing **a sealed, error-free, byte-identical
artifact stating a phrase the document does not contain**; then fixing that silently lost the rest of
the sentence; then the block counter advanced only for blocks that were read, so every address after a
footnote was one short.

**What this slice could not do, stated rather than skipped: no corpus of real files was available, and
no producer either.** That statement recurs for every ODF and RTF and EPUB slice, and it is why each
element set is read off the specification and kept narrow.

## S6 — ODS

**The address this format does not write, which is why it was a slice.** OpenDocument writes **no row
number and no column letter anywhere.**

| What the format writes | What this reader does |
| --- | --- |
| A table's name | Carries it verbatim, entities resolved. **The only component of a cell's address ODF writes down** |
| Nothing at all for a row or column | Counts position in document order |
| A repeat count on a cell | **Honours it** — the file saying *"and n more of these"* is a statement of position, so reading it is reading |
| A covered cell | Advances the cursor and yields no node: it occupies its columns and displays nothing |

**The column is a number here and a string in XLSX, and that is deliberate**: XLSX writes letters and
this format writes nothing, so each carries what its own file states.

**The frame-alternative rule, measured — and the result is not what the slice expected.** In a
spreadsheet a frame floats *over* the sheet: its words belong to no cell, **so there is no address at
which "one displayed phrase becomes one node" could be true.** Neither rendition becomes a node, both
are declared, and the anchoring cell keeps its own text.

## S7 — ODP

**The page that was free, and is still refused.** A presentation lists draw pages — discrete, ordered,
named, counted out loud by anybody describing a deck — with a master page's width beside them. **A page
record needed nothing computed at all, which had never been true before.** It is refused on the
conversion rule's own three words: *it invents pagination*. **A draw page is part of the presentation's
structure, not a page this engine measured.**

**This is where the PPTX argument stops transferring.** A deck in OOXML keeps each slide in its own
part; ODP puts every draw page in **one** part, so there is no part name to lean on — which is why this
locator counts draw pages as a position and says so in the field's name.

**The shape set is named, and the consequence is stated rather than hidden.** Two element types are what
a presentation writes for a text-bearing shape, and naming the set is what makes the shape component a
reproducible position. **No corpus of real files was available to measure which other elements matter**,
so the set is read off the specification and kept narrow, and text in anything else is declared.

**The frame rule, measured a third time — and the atom decides again:**

| Slice | The atom | A frame with two renditions |
| --- | --- | --- |
| **ODT** | The paragraph | First becomes nodes; the second is one declared erasure — the frame is anchored *in* a paragraph |
| **ODS** | The cell | **Neither** becomes a node; two erasures — a frame floats over the sheet and its words belong to no cell |
| **ODP** | The block inside the shape | First becomes nodes; the second is one erasure — **a presentation *is* a drawing, so there is nothing to float over** |

**The outcome matches ODT and the reason matches neither.**

**Speaker notes are the erasure rule inverted, so they are counted rather than spliced.** Notes are a
second stream and a master's text belongs to every slide, **so splicing either in would be a silent
*extra* rather than a silent drop — worse, because a consumer cannot tell it from evidence.**

## S8 — RTF

**Not a package, and that is what cost something.** An RTF has no parts, no manifest and no name for
itself. **The invariant grew a fourth rule instead**: a locator now answers whether it names a part, and
one that does not is checked differently. A constant part name would have let the existing rule run
unchanged **and would have been a string the document does not contain.**

**The page, said out loud.** A page break and a paper width in the plainest language any of these
formats use. Both are the producing application's print arithmetic.

**The count advances through destinations this reader does not read**, on the same reasoning every
package format needed: the address must reflect what a consumer counting in the file would find.

**Skip unless transparent — the inverted allowlist, outside XML.** The same rule ODT arrived at, applied
to a control-word stream. **One defect worth recording, because it made the whole rule silently off:**
the first version got the nesting wrong in a way that produced no error.

**A byte escape above 0x7F is declared, never guessed.** Its meaning depends on a code page this reader
does not read and carries no table for. **Emitting a Latin-1 character would be mojibake presented as a
success**, so each such byte contributes no character and is counted. Below 0x80 the byte is the same in
every ANSI code page and is read.

## S9 — EPUB

**The first time the no-pages law had to be argued rather than applied.** A navigation document may
carry a **page list**, and the older format's navigation control may carry page targets. **These really
do name the pages of a print edition** — real identifiers the file writes down.

It is refused because **a page record is a page with a width and a height**, and a publisher's label
about somebody else's paper has no geometry for anything to be validated against. It becomes a counted
region and `pages` stays empty.

**Reading order is the spine, and the archive is the trap.** The order lives in the package document,
not in the order the ZIP happened to store entries. **The fixture proves it rather than asserting it**,
by storing the second spine document first in the archive.

**No style sheet is read at all**, which is the general case behind three named gaps — text hidden by a
display rule is still in the record, generated text is not, and an image's alt text is absent because
attributes are never read as text.

**The whitespace engine is shared, and the three places it is not are written down.** The sharpest is a
line break between two CJK characters: CSS removes it and this reader turns it into a space. **The
widest gap this slice knowingly leaves, recorded rather than approximated** — the correct rule needs a
computed white-space value and the scripts on both sides.

**Numeric character references** are a valid part of XML needing no DTD, and the resolver that handles
them here was later adopted by every reader.

**Detection is exact rather than prefixed**: the stored entry's content must be exactly the declared
type. **The container rule is shared and the family is not** — an EPUB and an OpenDocument package share
a container convention and are different formats, so the family question and the container question are
asked separately.

## S10 — CSV, as an argued refusal

**The parse is not the problem. One field is.**

A CSV parse would fabricate nothing: every field's text would be bytes genuinely present, every record
ordinal a true line count, and an empty `pages` array simply true. **Exactly one thing would be false**
— the record would claim the file *is* a CSV when nobody measured that, and the identity structure has
two fields and **no room to say "asserted"**.

**The naive reason is the wrong one and is recorded here so it is not re-derived.** It is not that a
CSV parse would produce nonsense on prose. It is that **the engine could never tell a wrong assertion
from a right one, so fail-closed is unreachable from inside that design.**

**What shipped instead is the refusal, and it is a fallthrough rather than a CSV detector.** A `.csv`
is no longer told it lacks a PDF header, and a `.csv` and a letter containing a shopping list get
**byte-identical stderr** — **the test only an implementation that sniffed nothing can pass.**

**The reopening preconditions are written down, so a later slice inherits a decision rather than a
mood.** Either the identity structure gains a way to record asserted-versus-measured — a schema move
with its own scope document — or somebody builds a format predicate with a **measured false-positive
rate**, on a real corpus of prose and real-world CSV, stated the way the table gate states its number.
**Neither is met, neither is a day's work, and neither was started.**

**S10 closes v2's format row. It does not close v2.**

## S11 — Embedded assets, counted

**The measurement came first, and it found more than an omission.** No office asset was being read —
that alone would be an omission. What made it a defect is the declared-erasure rule: the media entries
were **uncounted, in no bucket at all.** One reader's unread-part prefixes did not match its media
directory, so a document with forty embedded images declared **zero** unread parts for them.

**It was also unexercised, so step one was a fixture and a failing test.** None of the three OOXML
fixtures contained a single media entry — **the blindness could not have been caught by anything that
existed.**

**The decision was a second bucket, not a wider one.** The existing code's message says its parts *carry
text*, and a picture does not. Rescoping it would have made one number answer *how much* for two kinds
of erasure, and **one number cannot honestly do that.**

**A media part is identified by where the package puts it** — the directories the package specification
itself names — **never by sniffing bytes and never by an extension.**

**The half this does not discharge is the owner's:** whether the roadmap row ever asked for more than a
count. Settled later as decision #17 — **counted satisfies v2**, and reading one is a contract change
rather than a reader change, because an office image has no page and no coordinate system.

## S12–S13 — Fuzzing and mutation for the office readers

**The obligation had stood open since v0**, and it is not hypothetical: an earlier slice's adversarial
review found a **panic** in a decoder.

**One fuzz target rather than eight, and the evidence that one is enough.** The read entry point is the
single one every format shares, and one measured campaign showed a corpus seeded with one valid package
of each shape reaches **all eight** readers through it. **Eight harnesses would divide one corpus eight
ways and explore each branch on a fraction of the budget.**

**The campaign ran 3,808,191 executions over about 57 minutes** under a memory sanitizer with debug
assertions, and **found nothing — which is a result rather than a pass.**

**Mutation testing took a second harness rather than a second manifest root**, because the existing
harness feeds every manifest entry to the PDF opener, so **office entries would have been refused as
non-PDF while every assertion still passed.**

**The office mutation kinds are not the same six as the PDF ones, and that is a result rather than a
shortcut.** Five carry over with their mechanics rewritten around the ZIP; one reduces to nothing for a
package and is **dropped rather than faked**; and five are new, because a container has hazards a byte
stream does not. **Every pair a kind cannot apply to is pinned and counted**, so a mutation that quietly
stops applying is a red test rather than lost coverage.

**The finding, and it is the reason to have built this:** the ZIP reader verified a part's declared
length and **never its checksum**, so a corrupted part that still inflated to the right size was read as
though intact. **Escalated rather than settled**, because it is a reader change with a cost and a
measurement — see S14.

## S14 — The checksum question, answered

**The measurement came first and it decided the shape.** The refusal shipped only after the
false-refusal rate was measured at **zero across 40 valid packages and 2,370 entries** — because
**refusing a valid archive would be a regression dressed up as a hardening.**

**The instrument was negative-controlled**, because a measurement that cannot detect a mismatch measures
nothing. And it was measured with **the engine's own decompressor**, which is not a detail: a mirror
implementation would have been measuring a different thing.

**The error is named** distinctly from a length or signature failure, so a caller can switch on the
cause.

**Detection does not verify, and finding that out cost a regression.** The first version broke routing:
the entry reader is called during *detection*, so a corrupt part made the router fail closed **naming
the wrong cause** about a document that plainly is what it says it is. **Measured, not reasoned** — the
corrupt file was built and run through the CLI before and after.

**Survivors fell from 36 to 31**, emptying the main-part class entirely. **The emptied class is pinned
rather than deleted**, so a mutant reappearing in it reads as a regression rather than as noise.

## S15 — A wire-string cluster that had been wrong at birth

**A minor bump, because two of the cluster's fifteen sites are emitted wire strings** — an artifact this
build emits differs from one the previous build emitted for the same bytes.

**The cluster moved as one, which is the whole point.** Repairing the comments alone would have left the
wire saying one thing and the comments another.

**What was wrong, and what was not.** The count was: there had been three detectors for some time, and
the strings said "neither". **The substance is true and stays true** — no detector reads the header tag —
and **the code is correct.** They were **wrong at birth, not rotted.**

**The new wording carries no ordinal**, so a fourth detector cannot re-rot it.

**The blast radius was measured before the wording was chosen:** the representation hash moves, the
profile hash does not, the oracle does not, and neither mutation harness does.

## S19 — The gate corpus, grown from four documents to twelve

**The unlock was in the method the whole time.** The four documents lived in a sibling tree that this
repository does not own. **A corpus you publish numbers about has to be one anybody can re-measure**, so
the new documents are committed here under a written admission rule.

**Admission is a rule now, not a judgement**, and personal documents are refused on two grounds rather
than one.

**The number: macro cell-slot F1 is 70‰ over twelve documents, where four read 64‰.** Read alone that
is stability. **The band refuses that reading:** 0‰..590‰, median 0‰, **ten of the twelve score exactly
zero**, two documents supply all 849 averaged points, and removing one drops the macro to 23‰.

**So the shape is bimodal, not weak-everywhere:** the ruled rule works where a producer drew the rules
and produces nothing where it did not. **That confirms on twelve documents the finding that reframed
five slices of work, and it is exactly the claim four documents could have produced by luck.**

**Fabrication is 0 across all twelve** — the one number with a required value.

**The new number is the cross-check**, which went from 0 to 9. One document detects **nine** tables
against four tagged ones, with dimensions up to 103 × 22 — phantom grids spanning whole pages,
contributing **11,295 false-positive slots.** Two things about it are easy to get backwards:

- **It is not fabrication.** The detector arranged real text into a grid that is not there; it did not
  invent text.
- **The engine already knows.** All nine disagreements are attributed to that one document, and there
  are exactly nine detected tables there — **so the cross-check is rejecting every one of them.**
  Nothing acted on it in this slice, because a rule that declined a table its own cross-check rejected
  is a **detector change**, and this slice was forbidden to make one.

**A gap closed as a side effect:** the document carrying the largest share of the gate number had no
manifest entry at all. An earlier slice pinned that gap rather than closing it and wrote that the day it
closed the guard would fail and bring whoever closed it back to that paragraph. **That is what
happened.**

**Decision #18 was written here and NOT decided** — the number is reported and the choice is the
owner's.

## S20 — The nine phantom grids the engine already rejected

**A minor that REMOVES output.** The ruled rule now declines a grid its own **structural** cross-check
rejects, rather than emitting it with a disagreement recorded beside it.

**Why the field beside the grid was not enough:** the Markdown and HTML projections draw **every** table
the artifact carries and consult no check. **So "keep and declare" delivered nine grids to a consumer and
delivered the contradiction to nobody.**

**Removing 11,295 false-positive slots moves the published macro by nothing**, which is the previous
slice's finding arriving in the other direction.

**The cost is twelve cell slots and every one is the empty string** — blank faces agreeing with blank
tagged cells. **No character of extracted text is lost anywhere in the corpus.** As a rate: 1,028 cells
emitted and 11,307 slots predicted, to get 12 right — **one per 941 wrong.**

**Only the structural half gates, and that was measured rather than reasoned.** Gating on the whole
check was built first and it **refused a 2 × 2 whose only defect is one edge sitting a single centipoint
out** — which is what a 1 pt stroked rule looks like. The two halves are not the same kind of statement:
the structural one is arithmetic on indices the rule assigned and admits no tolerance, while the
geometric one compares **exact** boxes against a lattice built *with* a tolerance, **so it fires on the
slop that tolerance exists to absorb.**

**The gate lives in one rule because it can only fire in one rule.** The other two build a cell for every
face from the same lines the table's box comes from, so their cells tile exactly and their cross-check is
fine by construction — **gating them would be dead code**, and a test runs both and asserts it rather
than a comment claiming it.

**What it cost: a shipped fixture changed its job.** It used to prove the engine emits a
self-contradicting grid and says so; it now proves the engine refuses one and says why.

## S22 — Why ten documents produce nothing

**A diagnostic that changes no detector.**

**What it found: the alignment rule emitted 0 tables across all 172 gold tables.** Every detection — 8
ruled, 9 stroke-ruled — is on the two documents that draw their grids. The four-document corpus had
recorded that finding; twelve documents make it a much stronger claim. **The engine ships a rule id, a
profile field and a slice of machinery that has never produced a table on a real document.**

**And the report names the precondition, uniformly.** On every gold page in all twelve documents the
alignment rule refuses at the same gutter floor: **the text's own columns sit closer than 12 pt, which
is prose spacing, not a table gap.** It never reaches the whole-page lattice that earlier analysis
described.

**The recommendation, named not taken.** v1's remaining gap **is not the alignment rule** — it has never
fired and cannot without lowering a floor the gold negatives prove is load-bearing, so retiring or
reworking it is a version-boundary question. **The gap is that the two working rules require the
producer to have drawn the grid, and the NIST producers draw their tables without one.** Recovering the
missed slots needs a derivation that reads geometry from the tags or from column inference — **a new
derivation class rather than a v1 tuning.**

**Micro recall is 4‰** — 70 of 15,755 gold slots. The framing offered to the owner is *"finds a table on
two of twelve documents"*, which is **the same corpus as 70‰ macro**.

## S24 — Tagged tables

**A minor, because a reader changed.** The five slices before it measured; this one ships a fourth
detection rule, emitting a table for each one the structure tree **declares** that no geometric detector
matched.

**The classes invert the usual intuition, and that is the argument.** A tagged table is `Extracted`
rather than `Computed`, **because the document stated the grid** — the engine read it rather than
inferring it.

**Geometry is typed-absent with no box invented**, under a variant that distinguishes *this source has
no box* from *the reader could not measure one*. **A tagged table's absent box is a property of its
source, not a shortfall of the reader.**

**The cross-check reports not-applicable, never `ok`.** It compares two derivations, and a tagged table
supplies only one. **Getting this wrong is the risk the slice named — a stray `ok` would be the check
passing a comparison it never ran.**

**Not groundable:** a table with no box cannot enter the grounding artifact, so the projection omits it
and counts and declares the omission.

**The two risks, and how each is answered.** Double-emission is prevented by emitting only where the
tree declares a table no detector found — a tagged table a detector *did* match rides as that geometric
table's own check instead. And the cross-check quietly passing everything is prevented by the
not-applicable result being pinned by a test.

**Measured over the twelve documents:**

| | Geometric (the gate) | Geometric + tagged |
| --- | --- | --- |
| Macro cell-F1 | **70‰** | — (the gate stays geometric) |
| Micro recall | **4‰** (70 / 15,755) | **502‰** (7,924 / 15,755) |
| Tables | 17 detected | + **157** tagged |
| Cells | 208 emitted | + 15,593 tagged |
| **Fabricated** | **0** | **0** |

**The geometric gate is untouched, and kept apart on purpose:** it scores the detectors against the
independent tree, and **a tagged table scored against the tree it came from would measure the tree
against itself.**

It does not reach 1000‰ because the tagged emit and the gold share the tree derivation but **the cell
text still comes from run joining**, which differs from the gold's join wherever a cell's runs do not
concatenate to the same string. **The band, the median and the gate verdict are unchanged, which is the
proof the detector did not move.**

---

## The repair slices

Eleven slices in this version repaired guards, prose and instruments rather than adding capability.
They are grouped because **the lesson repeats and is worth more than the individual findings.**

| Slice | What it found |
| --- | --- |
| **S9.1** | Erasure counters that could wrap — and that a site list is not a search |
| **S10.1 / S10.2** | Documentation that had stopped describing the code. The worked examples in two draft schemas published **two different digests for the same representation of the same document**, and at most one could ever have been right |
| **S12.1** | A CI job that did not build what it claimed to. Verified by breaking it: with the added line removed, the test fails and names the reason |
| **S13.1** | Guards that check nothing. One of fifteen tokens had been **dead for twenty-two commits** — and the repair is a rule, not a number |
| **S13.2** | The roadmap reorder, recorded rather than argued. OCR was **renumbered rather than only resequenced**, because a later build carrying a lower version defeats the one job that field has |
| **S13.3 / S13.5** | Statements that stopped being true. One document claimed to hold the settled decisions *verbatim, identically* while holding fourteen of seventeen. Two comments named tests that **have never existed** |
| **S14.1** | Those two tests, written. Each guard was **broken on purpose and watched go red** |
| **S16** | An instrument four slices had rebuilt wrong — **three of four wrong, each in a way the others could not see** |
| **S17** | A guard passing while covering four fifths of its subject. **A floor set one below its own population** |
| **S18** | `cargo fmt --check` had been red across four slices that each recorded a green — **because nothing ran it.** The finding underneath: no "green" in this repository had ever been a fact |
| **S21** | A mutation that had claimed to reach the file trailer and landed 373 KB short on the largest fixture. All eighteen of its survivors now fail closed |
| **S23** | Two coverage claims retired: one verified on the wire, one argued for deletion rather than papered over with a fixture |

**Four lessons these slices establish, and they generalise past this version:**

1. **A site list is not a search.** Several handed findings named some sites and missed others. Every
   later repair ran an independent sweep and reported its method, **so "five" is a result rather than a
   mood.**
2. **A guard that reads its own subject wrongly passes forever.** A floor derived from the population
   beats a hardcoded one; **an exclusion list has to be asserted complete or it goes hollow.**
3. **Break it on purpose.** Every repaired guard in these slices was watched failing before it was
   trusted. Several earlier ones had never been.
4. **A second copy of a table goes stale.** That is why the decisions live in one place now and the
   verify-boundary document points at them.

**Two of these slices also corrected a handed finding against the code**, and the code won. That is the
pattern worth carrying: **a finding is a hypothesis until the sweep runs.**

---

## Standing rules for every v2 slice

1. **No public confidence field** — including on an office node or in a summary string.
2. **No box, and no role, derived from a font size** — and **no page derived from anything**.
3. **No silent drop and no silent repair** — a dropped character is a named bucket with a count.
4. **No invented coordinate, identifier, fingerprint or page number** — pagination is the leading case
   here rather than the footnote.
5. **A gap is never presented as a success** — a quote that cannot bind fails closed.
6. **Byte identity across runs, and across formats.**
