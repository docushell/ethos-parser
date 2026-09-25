# Changelog

All notable changes to ethos-parser, newest first. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

**Seven versions are tagged, 0.55.0 through 0.61.0, and five of them carry binaries.** 0.55.0,
0.56.0 and 0.57.0 each ship the macOS pair — `aarch64` and `x86_64` — on the repository's GitHub
Release ([`RELEASING.md`](docs/RELEASING.md) §8). **0.60.0 is the first built on every platform it
ships:** Linux, Windows and both macOS architectures, one fingerprint across four runners —
delivered, unfortunately, as the single `release-bundle.zip` its own notes tell a reader to unzip.
**0.61.0 is the first to ship those four as five separate assets** — the four archives and
`SHA256SUMS.txt`, published 2026-09-25 — so a consumer downloads one platform and the manifest
instead of all four; §8's amendment records why 0.60.0's shape could not be repaired. **0.58.0 and
0.59.0 are tags with no release object**, and little is lost by that: 0.60.0 descends from both, so
their work is in the binaries above, and a reader who needs one of those two exactly builds it from
its tag under the pinned toolchain. Every earlier number is in-tree only. Nothing is on crates.io,
npm or PyPI.

**Every version moves `profile_sha256`**, because `parser_version` is a profile field — so artifacts
from two builds are correctly non-comparable even when nothing else changed. That is the mechanism
working, not a regression.

Entries through 0.1.0 are grouped by **milestone** rather than by version, because a milestone was
the unit of work that had acceptance criteria. The per-slice reasoning behind each entry lives in the
milestone documents ([`05`](docs/history/05-MILESTONES.md), [`09`](docs/history/09-V1-MILESTONES.md),
[`11`](docs/history/11-V11-MILESTONES.md), [`13`](docs/history/13-V12-MILESTONES.md),
[`15`](docs/history/15-V2-MILESTONES.md)); this file records what changed.

---

## [0.61.0] — the outline an author wrote down, and an ODF heading at the level it declared

**A MINOR, because readers changed.** A PDF catalog's `/Outlines` is read for the first time,
and an ODF `<text:h>` projects at the level its own element states — so on bytes 0.60.0 already
accepted, this build emits an outline record it emitted none of, and Markdown and HTML whose
heading levels differ. Three further changes move nothing and put on the wire what the tree
already knew and no consumer could read: **what kind of box** `measured_ink_boxes` has always
meant, **the bidi order** this engine never reorders, and **the document metadata** no reader
opens. The first two were open questions in the contract; the third was true and unsaid.

### The outline a document declares, and what kind of box this engine draws

**`/Outlines` is read** — `outlines-v1`. A PDF catalog's outline is a hierarchy the **author wrote
down**, so it is consumed rather than inferred, and `P14` — which refuses *role inferred from
presentation* — does not bear on a declaration. It lands as its own record on the representation,
never as `Node`s: a bookmark title is text no content stream painted, so it has no native locator
and North Star #4 requires one on every node. **The consequence is deliberate: an outline title is
not quotable.** `locate` and grounding both read `nodes`.

Measured on six gate documents carrying one: **2 273 entries**, per-document 1251/433/347/160/69/13
at declared depths 5/5/3/3/3/2, **zero unresolved destinations**. Every figure matches an
instrument that measured them before the reader existed.

**Four things it refuses.** A cycling `/First`/`/Next` is refused by name rather than followed or
truncated. The depth is the chain's own and is never renumbered. No section end is emitted — an
entry declares where one *begins*, and two entries in this corpus target a page *before* their
predecessor's, where an inferred end would run backwards. A title holding a byte in `0x80`–`0x9F`
is absent and counted, not guessed: that block is where PDFDocEncoding, Latin-1 and Windows-1252
disagree, and the table already in this tree is Windows-1252, which would turn `Backup –
Cryptographic` into `Backup … Cryptographic`.

**Nothing is dropped.** An unresolved destination and an undecodable title each leave the entry
emitted with that one part absent and counted. `outline-absent` says *this catalog names none*
where `capabilities.outlines` says *this profile looks*.

**The box now says what kind it is** — contract §5.3, open since the box existed, closed by
`text_box_rule: advance-over-font-envelope-v1`. No box changes shape. What changes is that the
artifact says what shape they were: the pen's advance along the baseline over the font's
ascent-to-descent envelope across it, which is **not glyph ink**. `capabilities.measured_ink_boxes`
is named for ink and says only that a box was produced — the same ambiguity row `L18` refuses in
another parser, carried here while refusing it there.

**Identity.** `REPRESENTATION_SCHEMA_VERSION` 0.6.0 → 0.7.0, `EXTRACT_SCHEMA_VERSION` 0.4.0 → 0.5.0,
and `profile_sha256` moves three times across these slices to `sha256:b41f27cab5…`, each move in the
move log with its reason. A capability flip is a **claim**, not a knob: an artifact from before
never opened the catalog and one from after either carries the entries or declares `outline-absent`.

### Right-to-left text: the artifact now says the order is the page's

A producer whose layout engine has resolved bidi draws Hebrew or Arabic in **visual** order, so a
run's text is the logical word reversed and `char_codes` carries the page's order beside it. This
engine applies no bidi algorithm anywhere — applying one would reorder characters no byte of the
page put in that order. The consequence belongs to the consumer: **a quote copied out of a viewer
matches this text, and a quote typed in logical order does not.**

Until now that lived only in `CAPABILITY.md`, in the fixture's manifest note and in two test
comments. All true, and none of it readable by anything consuming the artifact.
`right-to-left-not-reordered` is now on the wire, counting the runs affected.

**Document-scoped, and that is the difference from the two unconditional codes.**
`low-contrast-not-detected` and `document-metadata-not-read` ride every artifact because deciding
whether they *apply* would mean reading what the profile never reads. Here the reader already has
the text, so the condition is measurable — a document that draws no right-to-left scalar declares
nothing, which is what makes the declaration worth reading when it does appear.

**The test is a block test and the wording says so.** Unicode's `Bidi_Class` is the property that
answers *is this character right-to-left*, and this engine carries no Unicode character database.
The predicate reads three ranges — `U+0590`–`U+08FF`, `U+FB1D`–`U+FDFF`, `U+FE70`–`U+FEFF` — so it
also matches a few scalars in those blocks that are not themselves right-to-left; an Arabic-Indic
digit is `Bidi_Class` `AN`. Over-declaring a limitation is the safe direction, and the limitation
claims blocks rather than classes.

**What moves on the wire.** Measured old against new at the same version string over all 70 fixture
PDFs: **exactly one artifact moved** — `rtl-hebrew-visual-order` — and 69 did not. On that one,
nodes, pages, geometry and every other member of the assurance block are identical. `profile_sha256`
does **not** move: a limitation is not a profile field.

### The metadata nobody reads now says so, and the backend version is measured rather than typed

Three repairs, none of which changes what this engine extracts. Each closes a place where the
artifact — or the build behind it — asserted a discipline that nothing enforced.

**`document-metadata-not-read`, declared unconditionally.** Every format this engine reads carries
author-declared metadata — a PDF's `/Info` dictionary and `/Metadata` XMP stream, an OOXML
`docProps/`, an ODF `meta.xml`, an EPUB package document's `<dc:>` elements — and no reader opens
any of it for its values. That absence was in **neither** `CAPABILITY.md` table, among none of
`assurance.rs`'s codes, and behind no capability flag, while the same page closes with *"Everything
the engine could not do is **stated** — as a named limitation, a typed absence, or a counted
bucket."* It was the counterexample. The code is profile-scoped and unconditional, on
`low-contrast-not-detected`'s precedent: document scope means *declared only where it applies*, and
deciding that a document **has** no metadata would mean reading the metadata.

It rides every representation and every classification, all nine formats — the two artifact shapes
that carry an assurance block. **The Markdown, HTML and `locate` artifacts carry none**, so they say
nothing about it either way, and the `CAPABILITY.md` row says that rather than claiming every
artifact. `profile_sha256` does **not** move: limitations live in `assurance`, not in `Profile`. No
reader is built here, and the capability flag is deliberately deferred to the slice that ships one —
a flag claims *this profile looks*, and this profile does not look.

**The declared backend version is now the version the lock file resolves.**
`BackendIdentity::default` types `"0.44.0"` by hand while every manifest asks for
`lopdf = { version = "0.44.0", .. }` — a bare requirement, which is the caret range
`>=0.44.0, <0.45.0`. The literal was correct and nothing was keeping it correct: a `cargo update -p
lopdf` landing 0.44.1 would have moved the lock, moved the code that is built, and left every PDF
artifact declaring a backend it was not built with, with the whole suite green. The comment at that
literal already said *"Bumping the crate moves this string"* — a discipline, not a fact. The new
guard in `contract_invariants.rs` reads `Cargo.lock`, on
`no_renderer_has_entered_the_dependency_graph`'s precedent, and fails on the drift. The eight office
profiles never had this defect: their backend **is** this workspace, so they read
`env!("CARGO_PKG_VERSION")`. The PDF one has no such source.

**`ci/gate.sh` compares the oracle's commit instead of printing it.** The block resolved
`../ethos-oracle`, printed its short HEAD for a human to eyeball, and compared it to nothing; the
one identity test asserts only that `--version` exits 0 and is non-empty. `ci.yml`'s own comment
says why that is not an identity — *"`ethos --version` cannot tell two builds of one version
apart"* — and ends with a rule that lived in prose: *"move ../ethos-oracle to it before trusting
ci/gate.sh."* The gate now reads the 40-character `ETHOS_ORACLE_REF` out of the workflow, refuses an
off-pin or dirty checkout before step 1 rather than after a six-minute compile, and says which
sibling repository moved so *"the gate is red"* from `ci/release-preflight.sh` is not read as this
repository being red. An operator's own `ETHOS_BIN` still wins and is never second-guessed — the
test for it sits **above** the script's own export, which is the whole trick — and the bypass is now
announced rather than silent.

### The declaration, added after the projection

**A block the document called a heading that comes out as body text is now counted.** Until this,
the projection flattened it and the artifact said nothing at all — and
`docs/history/14-V2-SCOPE.md` §9's third standing rule is *"No silent drop and no silent repair"*.

Two codes in `coverage.structural_erasures`, not one:

- **`heading-level-unresolved-v1`** — the element stated no level. ODF makes `text:outline-level`
  optional and resolves an omitted one through an outline style in `styles.xml`, a part these
  readers declare unread on the same artifact. **A gap in what was read.**
- **`heading-level-unrepresentable-v1`** — it stated a depth neither target format has a place
  for. Markdown has six `#` depths and HTML six `<h>` elements. **Read perfectly well, nowhere to
  put it.**

`NON_TEXT_NODES_NOT_PROJECTED` draws exactly this line — *"Folding the two together would make one
count answer two questions"* — and here the difference is a date: a slice that opens `styles.xml`
drives the first toward zero on the same documents, and nothing will ever make `<h300>` an
element. One integer over both would tell a consumer deciding whether to wait for a better build of
this engine precisely the wrong thing.

Neither carries a `gfm-` prefix. The test, in `html.rs`'s own words, is not *"is the code named
`gfm-*`"* but *"does this projection commit that erasure"* — and `MCID_RUN_JOINS` already settled
the naming half: *"It is **not** a `GFM_*` code: GFM is not what causes it."* The names are
format-neutral on purpose: a DOCX `<w:pStyle>` whose built-in name lives in `word/styles.xml`
reaches `-unresolved-` with the same meaning and no rename.

**This ships under the same `markdown-blocks-v10` / `html-blocks-v10` as the projection above**,
folded into one identity move. Not one character of either projection changes here — but the
census *is* output, so `fixtures/office/presentation-pages/presentation.odp` does emit different
`ethos.markdown.v1` bytes than it did before either slice. One id rather than two because **no
release carries either half**: to every consumer that will ever exist the two are one change, and
an id that moved twice between tags would claim two comparability boundaries where there is one.
`profile_sha256` is `sha256:850e4fa3…`. Neither schema version moves — a code is a value inside an
existing key.

**What the fold costs, stated rather than hidden.** Between `d963edf` and the fold, `origin/main`
carried an intermediate `-v10` that projected ODF headings without counting the ones it flattened,
so two commits in this repository's own history emit different artifacts under one rule id — the
state a rule id exists to make impossible. It is accepted on one fact: **no artifact was ever
published under the intermediate.** No tag, no release, no distributed binary, source only. Anyone
who built `main` in that window and kept the output should re-run it.

**What counts nothing:** a `<text:p>`, whatever attribute rides along on it; a heading that
projected at its own depth; and a `<text:h>` whose text normalizes empty, which emits nothing at
all, so there is no paragraph it was flattened into.

### The projection

**A MINOR when it ships, because an emitter changed.** An ODT or ODP `<text:h>` now projects as
`# ` and `<h1>`..`<h6>` at the level the element itself states. Before this, every ODF heading
came out a bare paragraph: the reader put the *fact* of a heading on the wire and left
`text:outline-level` unread, so the projection had nothing to emit a depth from.

**The wire changes a 0.60.0 consumer sees:**

- **`markdown_rule` and `html_rule` moved to `markdown-blocks-v10` and `html-blocks-v10`**,
  together, for the reason they moved at `-v3` and `-v8`: the change is in `heading_level`, which
  both projections call. A bump rather than a new id — the evidence is the same as `/H2` and
  `<h2>` already carried, and what moved is how many formats can state it. **The same `-v10`
  also carries the erasure declaration above**, folded into one move: a consumer coming from
  0.60.0 sees `-v9` become `-v10`, once.
- **`OfficeParagraphAttributes` and `OfficeOdfShapeAttributes` each gain `outline_level`**, an
  optional integer, **absent** where the block stated no level. A block that stated none
  serialises exactly the bytes it did before the field existed, so no artifact of a document
  without ODF headings moves except through `profile_sha256`.
- **Nothing else.** No schema version moves, no declaration is added or removed, and no PDF
  document projects a different byte. `profile_sha256` is `sha256:850e4fa3…`.

**Three things it refuses, each of which would have been easier:**

- **An absent `text:outline-level` is not level 1.** ODF makes the attribute optional and a bare
  `<text:h>` takes its depth from an outline style in `styles.xml` — a part these readers declare
  they did not open, on the same artifact. A default here would put a number on the wire that came
  out of a part the record says nobody read, so a heading that stated no level projects as a
  paragraph. The ODP fixture is exactly this case and is tested as such.
- **A level past six projects as a paragraph.** ODF names no ceiling, so the reader carries
  `outline-level="300"` as written rather than clamping a document that is not broken; `#######`
  and `<h300>` are not things these formats have.
- **`capabilities.structural_locators` stays false.** The v2-S5 note that kept this field off the
  wire said reading it *"would put a document outline on the wire while `structural_locators` is
  false"*. An outline is a tree — a relation between nodes, which is what a structural locator
  addresses — and a level is one element's statement about itself, in the same grammar
  `OdfBlockKind::Heading` has been on this wire in since v2-S5. No parent, child or tree position
  is claimed.

**Not in scope, and why.** **DOCX** carries no heading at all and still does not: `docx.rs` has no
`Event::Empty` arm, and `<w:pStyle/>` and `<w:outlineLvl/>` are always self-closing, so they are
invisible to that reader as written — and `w:val="Heading1"` is a styleId, an author-chosen token,
where the built-in name ECMA-376 fixes lives in `<w:name>` inside `word/styles.xml`. Mapping the id
to a level without opening that part is matching a convention, not reading a declaration. **ODS**
reads the heading flag and discards it when the block closes, and `OfficeOdfCellAttributes` has
nowhere to put it, so a level there would be a qualifier outliving the thing it qualifies.

---

## [0.60.0] — one process from a document to its Markdown, and a block join the page's own space decides

**A MINOR, because an emitter changed.** Two things: `markdown` learned to take a source document
and run both stages in one process, and the undeclared block join learned that a gap narrower than
the space a page itself draws is not a word gap. The first changes no byte of any artifact; the
second changes the Markdown and HTML of every document whose fonts draw spaces and set type with
tracking, which is why both projection rule ids move. **The wire changes a 0.59.0 consumer sees:**

- **`markdown_rule` and `html_rule` move to `markdown-blocks-v9` and `html-blocks-v9`**, together,
  as they moved at `-v8`, because the join lives in `markdown` and `html` calls it. A document whose
  fonts draw no space at all projects byte for byte what `-v8` projected.
- **Nothing else.** No representation, extract, classification, grounding or locations artifact
  changes shape, no schema version moves, and no declaration is added or removed. `profile_sha256`
  is `sha256:53bb81a1…`.

**How it was measured.** Both changes were measured over the 200 opendataloader-bench documents
against the build before them, and the engine was profiled stage by stage before either was written
(`crates/ethos-parser-cli/tests/pipeline_cost.rs`, an ignored instrument). The full comparison,
including what was tried and reverted, is
[`docs/measurements/liteparse-head-to-head/README.md`](docs/measurements/liteparse-head-to-head/README.md)
§7.

### Added

- **`ethos-parser markdown --source <FILE>`** runs extraction and projection in one process. The
  artifact is **byte-identical** to `extract` piped into `markdown` — asserted on three fixtures in
  both projections, and checked by hand on 30 corpus documents — because it is the same two library
  calls under the same default profile. What it skips is serialising a 250 KB representation to
  JSON, writing it, reading it back, parsing it, re-verifying a fingerprint this process computed
  moments earlier, and a second process start. **Measured: 0.035 s to 0.020 s per document**, mean
  over 200 documents, at 9.4 MB of peak memory. The fingerprint is still verified on the record
  path, where the record came from a file this process did not write; that asymmetry is the point,
  and the equality test is what licenses it. `html` keeps its single input, which the equality test
  also asserts, so the asymmetry is deliberate.

### Changed

- **A gap narrower than the space the page draws is not a word gap.** `ink_sequenced` accepted a gap
  of at most 12 centipoints — a quantization epsilon sized for rounding — or one the reader had
  already filled with a space of its own. A page set with letter-spacing draws each glyph as its own
  run and leaves a few centipoints between the ink boxes, which is neither, so every letter became
  its own block: one bench document projected **944 blocks averaging 1.2 characters** where the page
  draws *"Once the slides are created"*. It now projects 48.

The measure is the page's own: the median advance of the runs it draws whose text is nothing but
whitespace, per font, per size. **It is not the gap epsilon 0.47.0 refused** — that would have been
a constant chosen to sit in a trough Latin has and CJK has not; this is measured per font per
document, and a font that draws no space supplies no measure and joins nothing new. The alternative
of falling back to one glyph's pitch was built and measured at NID 0.8696 against 0.8714, because it
swallows word spaces, and refused on that.

Over the 200 bench documents: **NID 0.8694 → 0.8714, MHS 0.3321 → 0.3353**, TEDS unchanged, 58
documents moved and none down.

### Not done, with the measurement that says why

Three gaps against another engine on that corpus stay open, and each was attempted or costed rather
than argued about:

- **Reading order** (0.8714 here). The repair
  [`reading-order-causes.md`](docs/measurements/opendataloader-bench/reading-order-causes.md)
  prescribes — peel a full-width band, then look for gutters in what remains — was built and
  measures **net −0.1042** over 4 documents. The 178 documents on the identity arm are not hiding
  columns; their content streams are out of order, and reordering those needs a rule that reorders
  on position alone, which `reading_order.rs` refuses in its header.
- **Headings** (0.3353 here). Decision #29's S5 font-weight clause was built: it reads the font's
  own `/FontDescriptor /Flags` ForceBold bit or a `/BaseFont` name saying Bold, and it buys MHS
  **0.3353 → 0.5198**. It costs both of §7.5's bounds — rate band 0.00%..10.15% against 5%, count
  bound breached on three documents — and the guard that would bound it has no gap to stand on: the
  bounded and breaching documents interleave on every share measured. Not shipped, twice measured
  ([`docs/measurements/headings/README.md`](docs/measurements/headings/README.md) §7, §7.1).
- **Tables** (0.1704 here). The 28 documents scoring zero are 2 whose truth transcribes a picture
  (OCR, which this profile refuses), 12 where a drawn grid was built and refused, 11 past the
  4,096-cell ceiling and 3 under the gutter floor. Fixing only the causes that cannot fabricate
  reaches about 0.60; going further needs the relaxation `table-gate-v1.md` records fabricating a
  table on five of eight gate documents.

---

## [0.59.0] — a tag this engine writes reads back as its own, a quote is located in a representation, and an untagged page's headings are read from its type

**A MINOR, because readers and emitters changed.** Three things are new. `tag` writes a structure
tree into an untagged PDF, and the reader reads that tree back as this engine's — `computed`, not an
author's, for as long as its `/EthosParser` attribute survives. `locate` answers where a string lies
in a representation. And an untagged PDF's headings are read from the type its lines are drawn in,
declared as such, and projected as `#` and `<h1>`. Beside them the reader refuses three ways a page
was lost without a word, says when the empty user password opened an encrypted document, and moves
the pen over a run it drops, and a DOCX reads one branch of an `<mc:AlternateContent>` instead of
all of them. **The wire changes a 0.58.0 consumer sees** each rest on a North Star decision of
2026-09-17 (#25–#31), except where a bullet says otherwise:

- **A `pdf_tagged` locator carries `derivation`**, `extracted` or `computed`, required and with no
  default. A 0.58.0 build refuses a representation that carries it (`unknown field derivation`), and
  this build refuses a 0.58.0 representation carrying any `pdf_tagged` locator — a run's, or an
  annotation's or widget's the tree cites by `/OBJR` — with `missing field derivation`. Both are
  exit 2 from the CLI as `engine: malformed representation: … [malformed]`, measured both ways
  against the 0.58.0 release binary, and a refusal from every MCP tool that takes a representation,
  before its fingerprint is checked. Office and untagged-PDF representations carry no such locator.
  The library's extract artifact moves the same way — its tagged runs' locators and its tagged-table
  records carry the field — so each build refuses the other's extract artifact of any tagged PDF.
- **A text run carries `inferred_heading` where the heading rule read its line as a heading**, and
  nothing where it did not. A 0.58.0 build refuses a representation carrying it, because text-run
  attributes deny unknown fields, and a library extract artifact carrying it too. A representation
  in which no heading was inferred carries no new key from this rule, and 0.58.0 reads it, as it
  read `leading-gap-two-blocks`'s.
- **The profile moves:** `struct_tree_rule` to `struct-tree-v2`, `markdown_rule` and `html_rule` to
  `markdown-blocks-v8` and `html-blocks-v8`, and two new fields, `locate_rule`
  (`locate-scalar-exact-v1`) and `heading_inference_rule` (`type-size-v2`), each
  `not-run-for-this-format` on the office profiles.
- **Five new declarations:** `structure-tree-engine-written`, `encrypted-empty-user-password` and
  `headings-inferred-from-type`, each on its decision; `block-subdivision-leading-gap-only` on every
  PDF representation, the block cut's own limits put on the wire, which no decision records; and
  `classify-reads-no-structure-tree` on every classification, this release's answer to the
  `classify`/`extract` disagreement #31 filed as a defect to fix. **Two rewritten details:**
  `untagged-structure-tree-absent`, on every representation of a PDF with no tree, because decision
  #29 made its old wording false; and `geometry-absent-not-groundable` where the only nodes without
  an ink box are of a kind that has none, a defect repair no decision records.
- **Two new outputs:** the `ethos.parser.locations.v0` artifact, and a PDF `tag` writes, stamped
  `ethos.parser.tags.v0`.

`schema_version` stays at 0.6.0 and the extract artifact's at 0.4.0, on the precedent of 0.55.0 and
0.58.0: a MINOR whose wire change is named here and refused by the parser, not a shape bump.
`profile_sha256` is `sha256:91a42807…`.

**How it was measured.** Every engine change but one was compared against the build before it at the
same version string, so each count below is that change's own; the empty-password declaration's zero
is a qpdf count of the corpus. The reader fixes were compared over up to 311 PDFs — the 58 engine
fixtures, the 8 gate documents, the 35 oracle fixtures, the 10 gate-zero documents and 200
opendataloader-bench documents — by exit code and artifact digest, and categorised where a byte
moved. The writer's round trip ran over 293 documents; the heading rule's false-positive bound was
measured on the 11 documents whose authors declare headings and its MHS band on the 107 of the 200
bench documents whose ground truth holds a heading; `locate`'s ceiling was measured at the ceiling.
The eight gate documents' representations, Markdown and HTML were compared byte for byte across the
heading rule, 1.7 GB and 350 MB of them. Office output changes in the two projection rule ids its
Markdown and HTML carry, which are stamped from the PDF profile (`OPEN-WORK.md` §6), and where a
DOCX holds an `<mc:AlternateContent>` with text in a branch, which no document in any corpus here
does — beside the digests and version strings every release moves.

### Added

- **`ethos-parser tag <pdf>`, the tenth subcommand: a structure tree this engine writes, and reads
  back as its own** (v2.2's second clause; decisions #23 and #25–#27). It writes one `/Document` and
  one `/Div` for each block of the reading-order cut, in reading order, each `/Div` carrying an
  attribute object owned by `/EthosParser` with `/Derivation /Computed`; the page's text operators
  are enclosed in marked-content sequences spliced into its content stream, with a parent tree and a
  stamp naming the source, the profile and the version. No `/MarkInfo` is written. It fills absence
  only: a document with a `/StructTreeRoot`, its own output included, is refused, and so is
  everything it cannot place — marked-content ids with no tree, inline or behind a named property
  list, a named property list that resolves to nothing, an encrypted document, a content stream
  under a filter other than none or `FlateDecode` or one that does not decode to its end, bytes its
  tokeniser cannot account for, an operator whose runs the cut put in two blocks, an object carrying
  `/StructParents` with no tree, a page left unread, a document with no block — each by name and
  before a byte is written. **Every output is extracted again before it is returned**, and a moved
  run, binding, declaration, element or parent-tree entry is a refusal rather than an output. Over
  293 documents — the 58 engine fixtures, the 35 oracle fixtures and 200 opendataloader-bench
  documents — **129 tagged and 164 refused**, 146 of the refusals bench documents that are PyPDF2
  page splits, which kept their marked-content ids and lost their tree. On the 129: `extract` binds
  41,208 of 41,209 runs `computed` under `Document/Div` (the other is page furniture); `ground`,
  `markdown` and `html` each equal the original's on 129 of 129; `grounding-check` finds all 129
  valid; Ethos `verify` agrees on the 122 it can compare; a second `tag` is byte-identical on 129 of
  129; and `qpdf --check` exits 0 on every output. `tag` is not offered over MCP or the SDKs (#26).
  **The guarantee is this engine's alone:** 0.58.0, run as `extract` over the same 129 outputs,
  reads every written `/Div` as an author's, and so will any reader that does not know the owner, or
  this one if a tool strips the attribute object — read a tagged PDF with this version or later
  where the difference matters.
- **The reader reads a tag this engine wrote as its own.** Attribute objects under `/A` and through
  `/ClassMap` are read; an element owned by `/EthosParser` must say `/Derivation /Computed` or the
  document is refused as malformed; its runs bind `pdf_tagged` with `derivation: computed`; the
  document declares `structure-tree-engine-written`, with the element count, the bound runs and the
  rule; and the projections read an engine-written sequence as no declaration, so a tagged
  document's Markdown, HTML and grounding are its untagged original's.
- **`ethos-parser locate <representation> --quote-file <FILE>`, the eleventh subcommand** — an
  `ethos.parser.locations.v0` artifact, an MCP `locate` tool and `locate()` in both SDKs (decision
  #30). It answers where a string lies in a representation: every occurrence, as node ids, character
  offsets in Unicode scalars and the representation's own geometry for the nodes touched. The match
  rule is `locate-scalar-exact-v1`, code-point-exact, on the profile and named on every artifact; a
  match may join runs inside one block of the reading-order cut and never across two. **Exit 0
  whether the string occurs or not, 2 when an input cannot be read or is refused — an empty quote,
  one over 16,384 bytes or not UTF-8, a representation that fails its fingerprint — and never 1**: a
  string that occurs nowhere is an empty answer, because an exit code meaning *not found* is one
  step from a verdict. The quote is read verbatim from a file, because argv cannot carry every
  string a representation holds. The MCP tool's summary is counts only; the SDKs call the binary
  rather than port the rule; the LangChain toolkits stay at three tools. **It emits no verdict, no
  boolean and no score, and its match rule is not the verifier's** — `docs/26-LOCATE-SCOPE.md` §4.2
  records the five places the verifier resolves a quote differently. **Measured:** past 1,000,000
  occurrences every locator is withheld; at the ceiling the artifact is 133,778,286 bytes (127.6
  MiB) and costs 503 MiB of peak memory and 2.09 s. On each of the eight gate documents, a 60-scalar
  line lifted from its own text occurs exactly once, and a call costs about 20 ms plus the
  representation read at 80 MB/s — the search is a rounding error beside it.
- **Headings read from type, on a PDF that declares no structure** (decision #29). A line whose
  every run with a measurable rendered em is at least six fifths of the document's body em is a
  level-one heading: the run carries `inferred_heading`, the document declares
  `headings-inferred-from-type` with the line count, the rule and the body reference measured, and
  Markdown writes `# ` and HTML `<h1>`. The body em is the larger of the most common rendered size
  and the largest size holding at least a twentieth of the body characters on at least ten lines, so
  dense small type that outweighs a document's prose is not taken for the body. It reads the
  rendered em and not `Tf`'s operand, which is one value on 87 of the 200 bench documents. It runs
  only on a document with no structure tree or one this engine wrote; wherever an author declared
  structure, nothing changes. **Measured** on the eleven documents whose authors declare headings,
  with the tree stripped: a false-positive rate of **0.00%..4.61%**, worst `cfpb-home-loan-toolkit`,
  and 147 false of the 253 lines it read as headings, most of them real headings the producer tagged
  `/P`; no document carries more false headings than its author declared except `nist-sp-800-218`,
  whose title — 10 lines on the cover and the title page, tagged `/P`, against 7 declared — the
  owner accepted. On opendataloader-bench MHS rises **0.0000 → 0.3321** (band 0.0000..0.9986, median
  0.1490, worst `01030000000001`, first by name of the 50 still at 0.0000); NID moves 0.8697 →
  0.8694, a text effect of the `# ` alone, and TEDS does not move. The rule leaves the eight gate
  documents, which all declare structure, byte-identical apart from their digests and rule ids.
- **`encrypted-empty-user-password`**, document-scoped, from `extract` and `classify` (decision
  #31): a document whose user password is empty was read exactly as a plaintext one, because `lopdf`
  authenticates the empty password and removes `/Encrypt` before this engine looks. The declaration
  says the bytes are ciphertext, that nothing was withheld, that `source.sha256` binds to the
  ciphertext, and that no permission is enforced. Of 311 PDFs, qpdf finds 1 encrypted and it needs a
  password, so no corpus artifact moves.
- **`classify-reads-no-structure-tree`** on every classification, for the disagreement decision #31
  filed as a defect to fix: classification reads no structure tree and interprets no text, so its
  exit 0 does not predict that `extract` will succeed — measured on `tagged-cycle`, which classifies
  cleanly and is refused by `extract`. `classify --help` says the same. 308 of 311 classify
  artifacts grow by exactly 554 bytes; the other 3 are refusals, which write no artifact.
- **`block-subdivision-leading-gap-only`** on every PDF representation: the block cut's four limits,
  which stood only in `blocks.rs`, are on the wire — whitespace only, no indent branch, 63.7% of
  real paragraph breaks found at 100% precision on the one gate document able to label them, and a
  block is not a paragraph. It adds 1,425 bytes to each representation.
- **A right-to-left fixture**, `rtl-hebrew-visual-order`, and a measured statement in
  `CAPABILITY.md`: a producer that has resolved bidi draws Hebrew in visual order, so the run's text
  is the logical word reversed, as the page draws it. No limitation fires. Whether an artifact
  should declare that is the owner's question (`OPEN-WORK.md` §4).

### Changed

- **`derivation` costs 3.2% (`irs-fw9`) to 4.7% (`nist-sp-800-218`) of a gate document's
  representation**, median 4.3%: 25 bytes per `pdf_tagged` locator, 41.2 MB of `nist-sp-800-53Ar5`'s
  1.04 GB. Every existing tagged representation carries it as `extracted`.
- **`untagged-structure-tree-absent`'s detail** said nothing is inferred and that a heading guessed
  from type would be indistinguishable from an author's. Both stopped being true with decision #29,
  and the detail now says what is, so every representation of a PDF with no tree, and everything
  projected from one, moves, whether or not a heading is found in it.
- **`geometry-absent-not-groundable` states the population it has.** Where every text run carried a
  box, it still opened on *"0 of N text node(s) … carry no ink box"* over three zero reasons. Such a
  document now gets a detail naming the kind — an annotation, a form field or an image — and no text
  sentence. 34 extract artifacts move, each 1,063 to 1,066 bytes shorter: the 26 bench documents
  0.58.0's census named and 8 engine fixtures.
- **An empty `/ToUnicode` destination is accepted, on measurement**, and `scalar_code_mismatch`'s
  documentation says that false does not mean codes and characters align one to one: an empty
  destination can offset a ligature's two scalars. Over 311 PDFs — 40,339 `bfchar` entries and 2,806
  `bfrange` rows — not one destination is empty, so no artifact moves.
- **`markdown-blocks-v8` and `html-blocks-v8`**, for the inferred heading: the only input that
  projects differently is a run carrying `inferred_heading`.
- **The Rust library's surface changes in ways that break source.** `MARKDOWN_RULE_BLOCKS_V7` and
  `HTML_RULE_BLOCKS_V7` are renamed `_V8`, and public structs gain fields — `derivation` on
  `PdfTaggedLocator` and the tagged-table record, `inferred_heading` on the text-run attributes and
  the extract's `TextRun`, and `Profile`'s two new rules — so a struct literal naming them stops
  compiling. `write_tags`, `TAGS_ARTIFACT_TYPE`, `locate` with its types and limits,
  `STRUCT_TREE_RULE_V2` and `HEADING_INFERENCE_RULE_V2` are new. Nothing is on crates.io, and
  whether a change of this kind needs more than a MINOR is open (`OPEN-WORK.md` §4).
- **`extract --max-pages`'s help** restates its sizing rule as `docs/measurements/memory-ceiling/`
  §15 re-measured it at 0.58.0.

### Fixed

- **A document with a page `lopdf` would read in part, or panic on, is refused by `extract`, naming
  the page** — exit 2 and no artifact, where the page was read as whatever came back — and
  `classify` counts nothing on such a page, and no longer aborts on it. Three shapes: an inline
  image with neither a colour space nor `/IM true` panicked inside `lopdf`'s decoder, and the
  release build aborts on a panic — an MCP server with it; the decoder stopped at the first
  operation it could not parse and returned what came before as the whole page; and a `FlateDecode`
  stream cut or corrupt part way inflated to what came before the damage and was returned as a
  success. Over 311 documents 0 of 622 `extract` and `classify` answers moved, and the cost on
  `nist-sp-800-53Ar5` is 32.46 s → 32.57 s. **The first corpus example arrived the next day:** of
  OmniDocBench's 981 born-digital pages, one is now refused where the build before wrote an artifact
  of a page whose 125,718-byte content stream was dropped after its fourth byte.
- **A run dropped at an undecodable code still moves the pen.** The loop returned at the code,
  leaving the text matrix where it stood, so every later run of the text object was reported a
  dropped run's width to the left of where the page draws it. Ghostscript 10.06 draws the later text
  in the same place whether the dropped code decodes or not, and the engine now reports it there. Of
  311 documents 19 move — 11 bench, 6 gate, 2 gate-zero — and of their 4,381,031 runs, 66,280 (1.5%)
  move, all horizontally: rightward by up to 274.86 pt (`nist-sp-800-53Ar5`), and on 7 documents
  also leftward, by at most 1.21 pt, where a dropped run's net advance is negative; no text, run
  count or advance changes, and no classify artifact moves.
- **A DOCX reads one branch of an `<mc:AlternateContent>`.** Every `<w:t>` was collected whatever
  its ancestry, so a phrase written once as a drawing and again as VML reached the artifact at two
  citable addresses. The first `<mc:Choice>` is read, the rest are counted in
  `office-parts-not-read` when they held text, and paragraph and run addresses still count the
  skipped branch, as a consumer counting the file's own elements would. No document in any corpus
  here carries the element, and no in-tree artifact moves.

### Build and release

- **`release-artifacts.yml` builds the Linux and Windows binaries** — and macOS on both
  architectures — on a `v*` tag or by hand, executes each over the eight gate documents where it was
  built, and labels a target `verified` only when every fingerprint is byte-identical across the
  runners. It publishes nothing. 0.59.0 is the first version it can build: 0.58.0's commit predates
  it. As first pushed the file was invalid on GitHub, which reads no `matrix` in a job-level `if:`;
  a `plan` job now filters the targets.
- **`cross-os-digests` and `cross-os-identity`** build the release engine on Linux, macOS and
  Windows and fail if the artifact digests differ by a byte. Their first run, at `791f0fe`, was
  green: 81 documents, 320 of 324 digests produced. They now cover 84 documents.
- **The two forbidden-token greps run in CI's `check` job**, so a push to `main` runs them.

### Not done

- **The heading rule's font-name clause** is not built: adding it to the size clause measured 91.7%
  recall at up to 15.76% false positives on a stand-in signal, the ink-box height — three times the
  bound — and it stays a conditional slice.
- **Decorative large glyphs** — private-use icons, a lone `$`, a drop cap on its own line — can be
  read as headings. Refusing a line with no letters is recorded and not built. **One level only**:
  every heading the bench labels is level one, and its evaluator flattens levels, so a second level
  could not be measured.
- **`tag` refuses 146 of the 200 bench documents**, all PyPDF2 page splits carrying marked-content
  ids without a tree — 98 refused on those ids, 48 first on a `/StructParents` key with no tree;
  that count is the owner's to weigh against `docs/23` §3.6 row 2. And 6 of 4,116 reals outside
  content streams, in 2 PyPDF2 documents, do not survive `f32` below the ninth significant digit
  when a tagged file is written.
- **`LZWDecode` and `ASCII85Decode` partial output** is still read on the extract path, as `lopdf`
  returns it; no page of any corpus here carries either filter (`OPEN-WORK.md` §6).
- **No committed fixture** carries an `<mc:AlternateContent>`, a scalar outside the Basic
  Multilingual Plane or a combining mark; unit tests hold those paths.

---

## [0.58.0] — every PDF span says where its text lies, and turned text is boxed along its baseline

**A MINOR, because readers and emitters changed.** Every span a PDF grounding artifact carries now
has `char_start` and `char_end`, so Ethos reports no `missing_char_offsets` on it. Text turned by
its text matrix or its CTM gets a box along its own baseline, where it had no box or an upright one
— `/Rotate` text too, though the only such run in any corpus runs past its page and so now carries
none — and a baseline along neither axis gets a new typed absence, `not_axis_aligned`. Word
spacing no longer reaches a two-byte code, a `TJ` gap writes a space only after text drawn since the
pen was placed, a Type 3 font's height is refused where its `/FontMatrix` vertical is not the
default, and `classify` and `overlay` read under the 2 GiB source ceiling. The reader changes answer
the six defects [`docs/22-WORD-BOXES-SCOPE.md`](docs/22-WORD-BOXES-SCOPE.md) §9 recorded: four
fixed, one refused rather than mapped, and one a documentation defect, corrected in the contract.
**Two wire changes a 0.57.0 consumer sees, both accepted by the owner on 2026-09-16:**
`capabilities.char_offsets` is true in the PDF profile and every PDF representation, and on a PDF
grounding artifact that carries spans; and a 0.57.0 reader refuses a representation carrying
`not_axis_aligned`, which no real document measured carries. `profile_sha256` is
`sha256:0a739a77…`, moved for the `char_offsets` flip and for the version.

**How it was measured.** Every engine change but the source-ceiling one was byte-compared across
`extract`, `classify`, `overlay`, `ground`, `markdown`, `html` and every exit code, over 322 to 351
PDFs — the gate documents, the engine and oracle fixtures, the gate-zero corpus, 200
opendataloader-bench documents, synthetic probes and, for the Type 3 change, Ghostscript, matplotlib
and Chrome Type 3 files. The source-ceiling change compared `classify` and `overlay` — exit code,
stdout and stderr — over the 86 committed PDFs. The rotated-text fix was compared against 0.57.0 as
released, and every other change against the build before it at the same version string, so each
count below is that change's own. Office output changes only in one limitation detail, beside the
digests and version strings every release moves.

### Changed

- **Every span in a PDF grounding artifact says where its text lies in its element's.** `char_start`
  is inclusive and `char_end` exclusive, counted in **Unicode scalars** — not UTF-8 bytes, not
  UTF-16 code units, not character codes — which is the unit `ethos.grounding.v1` slices an
  element's text by. The cursor counts every member of the block, a run with no box included,
  because its characters are still the element's; a space the reader synthesized counts as one. An
  artifact claims `char_offsets` only while it carries spans: **past the million-span cap it carries
  neither**, because the schema refuses offsets without spans, so the flip changes no byte of
  `nist-sp-800-53Ar5`'s grounding artifact, and `ground`'s notice on such a document now names
  `char_offsets` as false beside `spans`. On `irs-fw9`, Ethos `verify` returns all five checks with
  the status and evidence it returned before, and drops `missing_char_offsets` with the
  `capability_limited` warning it was the only cause of. `char-offsets-not-emitted` leaves the PDF
  profile's limitations; the eight page-less profiles keep `char_offsets: false`, and their detail
  for it no longer gives the spent `0..len` reason — the one limitation detail every Office
  representation changes; Office grounding artifacts are byte-identical at equal version. Against
  the build before it, over 328 PDFs: 324 grounding artifacts gained an offset pair on each of their
  2,447,419 spans and nothing else — stripping the offsets and resetting the flag gives the base
  bytes on all 324 — and no node, box, id or text moved anywhere. Recomputed out of tree from the
  base representation, 0 of the 2,447,419 disagree; 280,617 spans carry an offset pair a UTF-8 byte
  cursor would have written differently. Both checkers call all 324 `valid` and `matched`. **The
  cost: artifacts grow 29.9%** (251.6 MB → 326.8 MB over the 324; `nist-sp-800-161r1` 53.37 → 69.23
  MB; the largest 100.25 → 130.46 MB, none within 10% of the 256 MiB ceiling; no growth against
  0.57.0 as released, whose artifacts also lack the rotated-text fix's new spans, is recorded), and
  **validation costs more**, because both checkers collect an element's characters once per span:
  `grounding-check` on `161r1` 0.29 → 0.53 s, Ethos 0.72 → 1.07 s, and `ground` 2.72 → 2.87 s. The
  schema bounds the worst case: a synthetic million one-scalar spans in 61 maximum-length elements
  takes `grounding-check` from 0.55–0.81 s to 9.59–14.45 s, and Ethos from 1.32–1.92 s to
  10.66–15.80 s. **A 0.57.0 representation grounds without offsets**: `ground` follows the claim the
  representation itself makes, which 0.57.0 wrote as false, so re-extract to get them. Proven by
  `char_offsets_index_the_element_text_in_unicode_scalars`, where the real `extract` and `ground`
  write the artifact and the pinned Ethos decides; its negative control writes a byte cursor's
  offset and both checkers answer `invalid_offsets`. A MINOR.

### Fixed

- **Turned text gets a box along its own baseline, and a baseline off both axes says so (§9 items 1
  and 2).** A run's box was built from its advance, which reads only the text matrix's `e`: text
  turned or mirrored by its text matrix advanced 0 or less and was typed `no_ink_to_measure`, the
  absence that says nothing was drawn, and text turned by its CTM or `/Rotate` got an upright box
  laid along x. The box now follows the pen's travel as a vector through the text matrix, the CTM
  and `/Rotate`, on the side the glyph tops point; upright boxes are bit for bit 0.57.0's. Against
  0.57.0 — 340 PDFs, and `nist-sp-800-53Ar5` alone — 39 documents changed, `classify` and `overlay`
  on none, and no node's text, order, parent, attributes, findings or locator moved:
  - `no_ink_to_measure` → measured, every box vertical and every advance still ≤ 0: 31,699 runs on
    the seven gate documents, 4,685 on `nist-sp-800-53Ar5`, 30,773 on gate-zero and 74 on
    opendataloader-bench;
  - measured upright → measured vertical: 52,644, all `nist-sp-800-53Ar5`'s margin note;
  - → `not_axis_aligned`: 0 on every corpus measured; the new engine fixture
    `rotated-and-mirrored-text` and two probes carry it by construction;
  - measured → `measured_off_page`: conformance `rotation-90` and its gate-zero copy, the only
    `/Rotate` text in any corpus. Its turned box runs 1.04 pt past its page — Ethos's own layout for
    that fixture runs past it too — so it loses its wrongly horizontal grounding element instead of
    gaining a box, and the limitation's off-page sentence now says its origin is on the page.

  Grounding gains the newly measured runs — `nist-sp-800-207` 3,532 → 6,332 elements, `-161r1`
  30,202 → 47,412, `nist-sp-800-53Ar5` 49,525 → 54,172 — and every changed artifact is valid and
  matched under both checkers, with identical reports. Markdown and HTML differ only in
  `representation_sha256`; `ground`'s stderr count of omitted nodes moves by each document's
  transitions; `extract` on `nist-sp-800-161r1` and `-53Ar5` costs the same time and memory. An
  independent reader, run out of tree because it is AGPL (decision #14), counts the same vertical
  non-whitespace characters, page by page, as now sit in runs with a vertical box on the five gate
  and two bench documents it was compared on, and 60,043 of 60,066 on `nist-sp-800-53Ar5`, the 23
  being page 47 text neither build extracts. **New: `GeometryAbsence::NotAxisAligned`, wire
  `not_axis_aligned`** — measured, not a declarable limitation, not groundable. **Breaking in the
  way a MINOR allows:** `GeometryAbsence` is not `#[non_exhaustive]`, so an exhaustive match
  downstream stops compiling, and a 0.57.0 reader refuses a representation that carries it. **Not
  fixed:** `PdfLocator::advance` still measures along the text matrix's x before rotation, so on a
  CTM- or `/Rotate`-turned page it disagrees with the box, and every run above that went from
  `no_ink_to_measure` to a vertical box still advances 0 or less, so origin plus advance gives no
  extent there — a known defect, recorded on the field and in docs/22.
  `docs/measurements/rotated-text/` holds the instruments. A MINOR.

- **Word spacing reaches only a simple font's code 32 (§9 item 3).** A composite font's two-byte
  code `<0020>` took `Tw`, against PDF 32000-1 §9.3.3, so its run's advance and box were off by `Tw`
  and every later glyph on the line moved with it. Over 327 PDFs one real document moved: gate-zero
  `cfpb-home-loan-toolkit` page 17 shows `=` as `<0020>` under `-0.017 Tw`, so that run's advance
  goes 713 → 731 centipoints and the nine runs after it on the line move 0.187 pt right, onto the
  origins an independent reader draws them at. Its grounding re-boxes 10 elements and 10 spans with
  counts unchanged, and its Markdown and HTML differ only in `representation_sha256`. The other six
  documents that changed are synthetic probes: among them a lone zero-width `<0020>`, which had a
  `Measured` box made only of `Tw`, and negative `Tw`, which had typed a drawn glyph
  `no_ink_to_measure`. Nothing changed on the eight gate documents or the 200 bench documents, and
  simple fonts are bit for bit unchanged. A MINOR.

- **A `TJ` gap writes its space only after text drawn since the pen was placed (§9 item 4).** A `TJ`
  number at the space threshold wrote a flagged space onto whichever run was shown last, even after
  `Tm`, `Td` or another operator had placed the pen again, so a producer re-placing the pen on the
  same line got a space it never drew: 'i ncluded', 'Y OUR', 'Gar cía'. Over 322 PDFs, 60 runs in 6
  documents lose their space — `nist-sp-800-161r1` 44, `-171r3` 3, gate-zero
  `cfpb-home-loan-toolkit` 8, and opendataloader-bench `01030000000001`, `…02` and `…04` 2, 1 and
  2 — and 47 of the 189 synthesized spaces on the seven gate documents are gone. No box, id or count
  of nodes, elements or spans moves, and `classify`, `overlay` and every exit code are identical.
  Each of the 60 runs' `scalar_code_mismatch` goes true → false, and the Markdown and HTML coverage
  counts `whitespace-collapsed-v1` and `source_chars_in_representation` fall by each document's
  lost spaces. **Two tagged table cells in `cfpb-home-loan-toolkit` change text** ('“I f I lock' →
  '“If I lock', '“C an you' → '“Can you', and '“H ow' → '“How' twice), so
  `fixtures/labelled/table-truth.json` was regenerated: exactly those four spaces, and every table
  score is unchanged. **One Markdown and HTML block break is new:** `nist-sp-800-171r3`'s 'ad d'
  becomes 'ad' ‖ 'd', because with the invented space gone the declared path's ink-contiguity test
  refuses a 17-centipoint gap, the same test that already splits the rest of that word. Both forms
  read two words, the new one invents no character, and grounding reads the word whole. A MINOR.

- **A Type 3 font keeps its height only where its `/FontMatrix` leaves the vertical at the default —
  refused, not mapped (§9 item 6).** A Type 3 font's ascent and descent were read as thousandths of
  an em whatever its matrix said, so a 10 pt probe under a 0.01 matrix got a box 0.9 pt tall.
  Nothing in a Type 3 font says which units its descriptor uses, and producers disagree: LibreOffice
  7.5 and 7.6 write thousandths of text space under a 1/UPEM matrix, where carrying the envelope
  through the matrix would shrink an exact 12 pt box to 5.86 pt (read in its source, measured only
  on a probe built from it). So where the matrix's b, d or f is not the default, the font's runs
  that draw ink are `not_reported_by_reader`, and their advances are unchanged. Over 351 PDFs, 17
  documents changed, every one a synthetic probe; **no real document in any corpus moved**, and
  `ci/artifact-bytes.py` is identical over all 272 fixture artifacts. Withdrawn with the wrong
  boxes: a right one on a LibreOffice 7.5/7.6-era Type 3 font, which no corpus holds. The
  not-groundable limitation's "no `/FontBBox`" is false for these runs and is kept, because
  rewording it moves nearly every PDF artifact. A MINOR.

- **`classify` and `overlay` read their input under the 2 GiB source ceiling.** Both opened their
  PDF with `Document::open`, whose read has no bound, so the ceiling 0.56.0 put on every other
  path-reading command never reached them — the gap 0.56.0's entry named. On 0.57.0,
  `classify /dev/zero` held 5.2 GiB after two seconds and `overlay /dev/zero` 12.8 GiB, both still
  reading when killed. Both now read as `extract` does: a file over 2 GiB is refused from its
  metadata without being read, and a source with no size is refused one byte past the ceiling, exit
  2, `resource_limit`. Exit code, stdout and stderr are equal to 0.57.0's on all 86 committed PDFs,
  and peak memory is within noise. The library's `Document::open` is unchanged. A refusal where
  0.57.0 read without limit, so a MINOR on its own.

- **The contract describes the box this engine emits, and a ligature was never the defect (§9 item
  5).** `docs/01-CONTRACT.md` §5.3 said the engine *"emits measured ink boxes only, from the
  embedded font program or the font descriptor"*. The box is the font's ascent-to-descent envelope
  stretched over the pen's travel — the construction §5.3 itself names as the one to avoid — and its
  sources include `/FontBBox` and the standard-14 AFMs. §5.3 now describes that box, and records,
  without deciding, that nothing on the wire declares a box's kind and that §6's versioned rule for
  it is not in the profile; §5.1, §6, §10, `docs/CAPABILITY.md` and the doc comments that repeated
  the claim are corrected with it. A run holding a code that decodes to several characters is its
  own advance wide, because each code advances the pen once by its own width: all 29 in
  `nist-sp-800-53Ar5` (26 exactly, 3 within the centipoint that quantizing two edges allows) and all
  608 in 79 of the 200 opendataloader-bench documents (473 exactly, 135 within it). The conformance
  ligature run's advance is now pinned at 9600, and a unit test pins one advance per code. **Not
  pinned: the box's own width.** No fixture carries a measured box on a multi-character code, so a
  reader spanning the box over the run's letters would pass every test and is caught only by
  `docs/measurements/word-boxes/ligatures.py`; closing it needs a new fixture. **0.55.0's entry was
  wrong the same way:** it said `scalar_code_mismatch` flags exactly a code that decodes to more
  than one character, but it compares two counts, so a synthesized character sets it too — 15,164 of
  the 15,772 flagged bench runs are synthesized-only. No emitted byte moves: 323 PDFs are
  byte-identical against the build before it. A PATCH on its own.

### Not done

- **Word boxes are refused, on measurement**
  ([`docs/22-WORD-BOXES-SCOPE.md`](docs/22-WORD-BOXES-SCOPE.md) §7 names what reopens them). The
  pinned verifier resolves a quote to the element before any span, so a word box changes a result
  only for a claim naming its `span_id` or a page-only `value` equal to the word, and every claim
  this repository verifies cites by element.
- Left open, each recorded in [`docs/OPEN-WORK.md`](docs/OPEN-WORK.md): `advance` measured before
  rotation; a run dropped at an undecodable code stopping the pen short, so later runs on its line
  sit left of where they are drawn; a code whose `/ToUnicode` destination is empty decoding to no
  character, which can hide a ligature from `scalar_code_mismatch`; how a text-run box declares its
  kind, and whether the liteparse refusal's Wall 2 and §5.3's "wrong for a citation highlight"
  should be re-taken now that both apply to this engine's own box;
  `docs/draft-schemas/derivation-class.draft.json` still saying "ink boxes from font metrics"; the
  semver policy for `GeometryAbsence` growing without `#[non_exhaustive]`; Office Markdown and HTML
  stamping the PDF default profile's `profile_sha256`; and the table accuracy print reading 69‰
  where `table-gate-v1.md` records 70‰, a gap older than this release.

---

## [0.57.0] — `grounding-check` answers as Ethos does, in a third of the memory

**A MINOR, because a reader's output changed.** `grounding-check` gives Ethos v0.6.0's report byte for
byte on a valid artifact, and its verdict, error code and path on an invalid one, on every input
measured; on 198 of 3,681 mutated artifacts 0.56.0 gave a different code or path. **Every artifact
0.56.0 judged valid gets a byte-identical report** — 162 of 162 in that corpus — unless it is bound
to a source over 256 MiB (below). Of the 3,519 invalid ones, 1,352 keep their report byte for byte
and 1,969 keep code and path but not `message`: 1,379 fixed sentences now quote the parser
(`array limit exceeded at line 1 column …`), 492 type and missing-field errors lose their
`at line … column …`, and 97 repeated fields read `duplicate object key` where they named the field.
`profile_sha256` is `sha256:de706c10…`, moved by the version alone.

**And lighter.** On a valid artifact `grounding-check` peaks at 3.8–4.1× its input where it peaked at
13.2–13.5×, and is 44–47% faster; one refused while scanning, as 53Ar5 is for its spans, now costs
about its own size. The oracle pin moved to Ethos v0.6.0, so the test suite checks page-less
artifacts against the pinned verifier for the first time.

### Fixed

- **`grounding-check` answers as Ethos v0.6.0 does, fault for fault.** It re-implemented
  `parse_grounding_json` from its rules rather than its walk, and differed wherever the rules did not
  decide the order. The single faults 0.56.0 reported differently:
  - an integer at `i64::MIN`: `limit_exceeded` became `invalid_field`, `invalid_invariant` or
    `invalid_bbox` in a release build, because `i64::abs` wrapped — and a debug build panicked;
  - an array past a million items was `limit_exceeded` at `/tables/N/cells` for cells, rather than at
    `/`; past a million spans, elements or pages only the message changed;
  - nesting 128 levels deep or more was `invalid_json`, not `limit_exceeded`;
  - a fixed-length array with an extra item, such as a five-number `bbox`, was `invalid_json`, not
    `invalid_field`;
  - a type error quoting a string that contains `EOF`, `trailing` or `invalid unicode` was
    `invalid_json`, and one quoting `unknown field` was `invalid_field` where Ethos, which classifies
    by the error's text, says `unknown_field` — this follows Ethos, quirk included;
  - a repeated unknown key was `unknown_field`, not `duplicate_key`;
  - an artifact written as a JSON array without `spans` or `tables` was `invalid_field` where Ethos
    reports it valid, or its real fault;
  - in a `cargo test --workspace` build, of two unknown keys the one first in the bytes was named,
    not the one first in sorted order.

  So were artifacts with two faults where 0.56.0 named the one its rules checked first — a million
  and one items under an unknown key was `unknown_field` — and Ethos names whichever the bytes reach
  first. The checker now runs Ethos's walk: one visitor holds its value rules and messages; a scan
  that builds nothing, then the typed parse, still answers every artifact that parses; one that does
  not is answered by a pass that names the first refusal without building, or, when there is none, by
  Ethos's build with each object's keys sorted the way its map iterates. Library callers of
  `grounding_check` get the same codes, paths and messages, and **`GroundingSource`'s `spans` and
  `tables` now default when absent**, as Ethos's do, so an array-form artifact without them
  deserializes where it failed — a reader change. Against Ethos v0.6.0: the 198 of 3,681 disagree no
  longer, and no input that agreed disagrees now; 19 of 1,645 page-less and PDF mutations became 0 of
  1,866; an adversarial search built more than 25,000 inputs to break the claim and found no
  divergence in the checker. Fifteen cases are pinned in a unit test with every expectation read from
  `ethos grounding check`, and eight join the oracle's adversarial test, which fails against 0.56.0's
  code. Not copied: Ethos's `grounding check <path> -V` exits 0 without checking anything. A MINOR.

- **`grounding-check` reads `--source-artifact` when Ethos does: only for a valid artifact.** 0.56.0
  read the source first, under the 2 GiB source ceiling, so an invalid artifact with a missing or
  unreadable source got no report and exit 2, one whose source blocks on read got no answer, and a
  source between 256 MiB and 2 GiB was hashed and bound. Now an invalid artifact is reported, exit 1,
  without its source being read, and a valid one is refused with no report, exit 2, for a source that
  cannot be read or is over 256 MiB, Ethos's `max_file_bytes`. **New for library callers:**
  `grounding_check_reading_source` takes the source as a callback and calls it only for a valid
  artifact; `grounding_check` is unchanged. Checked against Ethos with a missing source, a sparse
  256 MiB + 1 source beginning `%PDF-`, and a FIFO held open and never written. A MINOR.

### Changed

- **`grounding-check` builds no `serde_json::Value` for an artifact that parses.** The tree its value
  checks walked was the peak, at 13.2–13.5× a valid input. Against 0.56.0, medians of five interleaved
  runs:

  | grounding artifact | peak RSS | wall |
  | --- | --- | --- |
  | nist-sp-800-171r3, 15.3 MiB | 205.9 → 62.2 MiB | 0.19 → 0.10 s |
  | nist-sp-800-37r2, 37.4 MiB | 492.5 → 142.8 MiB | 0.42 → 0.22 s |
  | nist-sp-800-161r1, 47.8 MiB | 639.4 → 194.3 MiB | 0.55 → 0.30 s |
  | nist-sp-800-53Ar5, 151.4 MiB, refused for 1.6 million spans | 1,922.8 → 153.9 MiB | 1.36 → 0.45 s |

  An artifact refused while scanning builds nothing either — 161r1 with junk after it, 540.3 →
  50.3 MiB — but one that scans cleanly and fails its typed parse still builds the tree, now Ethos's
  sorted and duplicate-checked one: 161r1 with an unknown key at its root peaks at 540 MiB, about 11×
  its input, as before, and takes 0.30 → 0.45 s. `docs/measurements/memory-ceiling/` §12 records
  these runs, made with `gcheckab.py`, and the `gcdiff.py` corpus behind the counts above. A PATCH on
  its own.

- **The oracle is Ethos v0.6.0, `8adda91`.** `ETHOS_ORACLE_REF` moved for the first time since it was
  set, from `5cb9f9b`, which predated grounding schema 1.1.0 and refused every page-less artifact this
  engine emits — `unknown_field` at `/elements/0/locator` — while the engine's checker accepted them.
  `oracle.rs` now checks all 16 committed Office documents, covering the eight page-less media types,
  page-less G2, and eight 1.1.0 rules broken one at a time; each fails against the old pin. Closes the
  page-less half of the "not covered" note on 0.56.0's G2 entry; the half about hand-built ids and
  overlapping cells stands. `ethos --version` prints 0.6.0 for both builds, so `ci/gate.sh` now also
  prints the commit `../ethos-oracle` is at. No output change; a PATCH on its own.

---

## [0.56.0] — the grounding schema's limits kept, and MCP safe to hand a path

**A MINOR, because an emitter changed.** `ground` used to exit 0 with an artifact the verifier refuses
for any document past one of `ethos.grounding.v1`'s limits other than the span cap; it now omits,
withholds or refuses, and says which (G2). The LangChain `ground` tool in both SDKs returns
`ethos-parser mcp`'s own summary, which had miscounted omissions since 0.49.0. Every artifact inside
the limits is byte-identical to 0.55.0's at equal version. `profile_sha256` is `sha256:73998…`, moved
by the version alone.

**And faster, and harder to break.** SHA-256 runs on the CPU's SHA-2 instructions on Apple silicon,
9–13% off `extract`, `ground`, `markdown` and MCP `node_get` as measured. MCP remembers which exact bytes already verified, so repeat calls on one
representation are 52–66% faster. A path a model names can no longer hang the server or exhaust its
memory, and an inline representation is no longer copied three times.

### Fixed

- **A path that is not a regular file no longer hangs MCP or exhausts memory.** The 2 GiB source
  ceiling was checked against metadata, which knows only a regular file's size, so `/dev/zero`, a
  pipe or a device was read without limit — the MCP server reached 2.8 GiB in three seconds and
  kept going. The read itself is now bounded one byte past the ceiling for `extract`, `markdown`,
  `html`, `ground`, `grounding-check` and MCP; `classify` and `overlay` read through
  `Document::open`, which has no ceiling, and are not covered here. CLI pipes and process
  substitution still work, and `ground` on the largest gate representation is unchanged
  (+0.7 MiB, byte-identical). In MCP, where a model names the path, a path that exists and is not a
  regular file is refused before it is opened — on Unix; Windows reports anything but a directory as a
  file: `/dev/stdin` read the server's own protocol stream and
  a FIFO with no writer blocked the open forever. **An inline `representation` is no longer copied
  three times**: the request is moved into the tool rather than cloned, and `ground` on an 82 MiB
  representation passed inline peaks at 1,413 MiB where it peaked at 4,867, with an identical reply
  (by path it is 276 MiB either way). Not done, deliberately: a cap on request line length — the host
  frames and holds every line, so any cap chosen here would be arbitrary. A MINOR on its own — MCP now
  refuses paths 0.55.0 read, and a source over 2 GiB with no size is refused where 0.55.0 read it;
  the section is already one for G2.

- **`ground` kept inside `ethos.grounding.v1`'s other limits, which it never looked at (G2).**
  0.55.0 enforced the span cap and nothing else, so a document past any other limit still exited 0
  with an artifact the verifier refuses whole. Every limit a record this engine wrote can reach is
  now kept, never by truncating. **An element whose text is over 16,384 bytes** — a paragraph of
  short runs joined past it, a spreadsheet cell of 32,767 characters — or a page-less element whose
  locator is over 2,048, which a crafted part or sheet name reaches, **is omitted** with its spans;
  in a PDF artifact later element ids close up behind it, and a page-less id is its node's and does
  not move. **More than 100,000 tables, any cell's text over 16,384 bytes, or a grid of more than a
  million cells withholds every table**, declared by `capabilities.tables: false` exactly as G1
  declares spans. **More than 5,000 pages, more than a million elements — counted after omission —
  or an artifact over 256 MiB including `ground`'s trailing newline is refused**: exit 2,
  `resource_limit`, no artifact, because the schema has no capability to declare a missing page and
  a partial set would be a silent hole. A producer string over the limit, which this engine never
  writes, is refused as malformed. What was omitted or withheld is declared on stderr with its
  counts, in MCP's `ground` summary, and to library callers in `Projection.elements_omitted` and
  `Projection.tables_withheld`; `OmissionReport` and `is_lossy()` are documented as the geometry
  ledger they always were. Comparisons are the checker's own — UTF-8 bytes, strictly greater — with
  the artifact ceiling one byte stricter for MCP and library callers, who get no newline. **Every
  artifact inside the limits is byte-identical**: `ci/artifact-bytes.py` equal over all 268, and the
  13 fixtures the schema-conformance test grounds are asserted to engage no degradation. Proven on
  tables this engine detected, re-sealed with a run and a cell past the limit: the engine's checker
  and the Ethos verifier both accept the degraded artifact, and both refuse it with the over-long
  text placed back in an element. The caps are one internal value, so tests exercise the element
  and page caps at their call sites, and the table, cell and grid caps in the function that decides
  them; fourteen wrong implementations were each shown to fail a
  test — among them `>=` for `>`, characters for bytes, a run's text measured instead of its
  block's, the element cap counted before omission, an id gap, an empty `tables` array for an absent
  one, one table withheld of several, and the newline left out of the artifact ceiling.
  **Unlike withheld spans or tables, an element omitted for its length leaves no trace in the artifact
  or the representation**: a pipeline that keeps only the artifact and discards stderr cannot tell.
  `docs/01-CONTRACT.md` §11 names the second omission reason — a length against a published limit,
  never a judgement of the text. New public items `ElementsOmitted`, `TablesWithheld`,
  `ELEMENTS_OMITTED_OVER_LIMIT`, `TABLES_WITHHELD_OVER_LIMIT`, frozen and documented. **Breaking for
  library callers:** `Projection` gains `elements_omitted` and `tables_withheld` and is not
  `#[non_exhaustive]`, so a struct literal or an exhaustive pattern stops compiling; `project()` newly
  returns `EngineError::ResourceLimit` for pages and elements, and the artifact ceiling is enforced by
  `to_canonical_bytes`, not by `project()`.
  **An emitter and exit-code change, so a MINOR** — although the only outputs that move are ones no
  verifier accepted. Closes the "not covered" note on 0.55.0's G1 entry. **Not covered:** a
  hand-built record can still carry an id past 256 bytes or overlapping cells, which no limit a real
  document meets reaches; page-less artifacts are checked against the engine's checker only, as the
  pinned Ethos predates schema 1.1.0. The SDK LangChain adapters' `ground` summaries are fixed below.

- **The LangChain `ground` tool's summary is `ethos-parser mcp`'s own, in both SDKs.** Since 0.49.0,
  when an element became a block, the Python and Node adapters reported nodes minus elements as
  "omitted for having none" — wrong on every document whose blocks merge runs: `irs-fw9` said 763
  where the engine omitted 127, `nist-sp-800-207` 87,472 where it omitted 8,095. They also lacked
  0.55.0's G1 clause and G2's two, and called an element omitted for its length one with no box.
  Their byte-for-byte pin never saw any of it, because both of its fixtures were single-run blocks
  with nothing omitted. The tool now makes one `tools/call` to `ethos-parser mcp` **by path, never
  inline** — the server holds an inline argument several times over — and returns that reply's text
  and `structuredContent`, so the words are MCP's by construction, including any clause added later.
  On a refusal it runs `ethos-parser ground` on the same path, so the error raised is `ground()`'s
  own. MCP's sentence is now built in one function, `ground_summary`, whose exhaustive destructure of
  `Projection` fails to compile when a field is added, and a unit test pins the full sentence with
  every clause firing — the first pin anywhere of G1's wording. New pins in both suites:
  `untagged-shredded-line` and `stroke-ruled-field-boxes` against MCP, re-sealed records past the
  string limit on `markdown-two-blocks` and `ruled-table-grid`, a reply carrying U+2028, the refusal
  raising `ground()`'s error, a shim proving one `mcp` run with a small request, and a scan that the
  adapters no longer format the sentence; five fail against the old adapters. Node's process
  machinery moved into a private `src/engine.js` so the tools can reuse it without a fourth export —
  with `run` taking optional stdin and `ground`'s temp-file handling extracted as
  `withRepresentationPath` — and three source scans were repointed at it: two would have passed
  vacuously on `index.js` alone, and the `maxBuffer` check, which had been matching a doc comment
  rather than the option, would have gone red. `ground()` in both SDKs now passes `--` before the
  path, so a path beginning with `-` is a path to the CLI as it is to MCP.
  `ci/sdk-suites.sh` now prints a failing Node suite's log instead of exiting before it. The ground
  tool's own failures change where they must: a timeout or a stdout ceiling names `mcp`, a refusal
  costs a second run, a path that is not valid Unicode fails as a JSON-RPC error in Node, and in
  Python a path whose filesystem bytes are not UTF-8 is refused by name rather than sent as a different
  file. The tool's output moves for the same input, which is a MINOR on its own; the section is
  already a MINOR for G2, and no version string moves in this change.

### Changed

- **SHA-256 uses the CPU's SHA-2 instructions on Apple silicon: `extract`, `ground`, `markdown` and
  MCP `node_get` are 9–13% faster.**
  `sha2` 0.10 enabled the ARMv8 backend only behind its `asm` feature, which this workspace never
  set, so every `aarch64` build hashed in portable code — and every `extract` hashes the payload to
  seal it and every read command hashes it again to verify it. `sha2` 0.11 detects the extension at
  run time. Interleaved on the largest gate document: `extract` 9.37 → 8.18 s (−12.7%), `ground`
  11.02 → 9.94 s (−9.8%), `markdown` −8.7%, MCP `node_get` about −11% per call; −9 to −13% on the
  smaller documents. Memory does not move. **Byte-identical** over all 268 artifacts, and an
  `x86_64` build under Rosetta 2 agrees with native. 0.11.0 was already in the dependency graph
  through `lopdf`, so this adds no crate and removes seven the old version needed. `docs/measurements/memory-ceiling/`
  §13, which also withdraws §12's statement that 0.10 used those instructions. A PATCH.

- **MCP stops re-verifying a representation it already verified: repeat `ground` and `node_get`
  calls are 52–66% faster.** Verification rebuilds and hashes the whole payload, and it was two-thirds
  of a `node_get` — 5.5 s of 8.3 on the largest gate document. The server now remembers the SHA-256
  of every path-form buffer that parsed and verified, and a call whose own bytes hash to one of them
  skips `verify_fingerprint` and nothing else: it still reads, parses and structurally checks its
  bytes and answers from them. Not the path, inode, size or mtime — a same-length rewrite with its
  mtime restored is verified again — and not the declared fingerprint. Inline input is verified every
  call. At most 64 digests, about 6 KiB; no document, tree, path or buffer is kept, and no argument or
  reply can name a digest. **Every reply is byte-identical to a fresh server's**, which a stdio test
  replays across a whole session against one-shot servers. Interleaved on 53Ar5: first sight of a file
  +2.3%, later `node_get` calls 12.78 → 4.86 s (−61.9%; −65% on the smaller
  documents), later `ground` calls −52.1%. **One pre-set line was breached, and it ships by the owner's decision:** back-to-back
  calls on the 950 MiB representation peak ~650 MiB higher (4823 vs 4174 MiB RSS), because each call now
  reads the next file before macOS has returned the previous call's freed tree — at 4 s between calls
  it is +290 MiB, at 8 s zero, and the smaller documents are unaffected. `docs/00-NORTH-STAR.md`
  decision 24 amends v1.2's *"not a session"*: no answer depends on an earlier call, only its cost.
  One bit leaks through latency — that these exact bytes were verified earlier in this process.
  Keeping the verified tree, for millisecond repeat calls, was refused. Eight wrong implementations —
  a length-only key, a prefix or head-and-tail hash, hashing before parsing, inserting before
  verifying, skipping whenever the ledger is non-empty, FIFO eviction, a ledger per line — each fail a
  test. `docs/measurements/memory-ceiling/` §14. A PATCH.

---

## [0.55.0] — the cut's horizontal half reaches the wire, and the first version released

**The first release.** Tagged `v0.55.0`, with `aarch64` and `x86_64` macOS binaries — each built,
executed on this host and compared byte for byte against the native build by
`ci/release-artifacts.sh` — and nothing on a registry. Linux and Windows are not shipped: nothing
here can execute them, and a binary nobody has run is an untested claim for an engine whose product
is byte-identical reruns. `profile_sha256` is `sha256:daada698…`.

**A MINOR, because readers and emitters changed.** Every PDF text run carries the `block` its page's
leading-gap cut placed it in, where the cut opened more than one; the ruled table rule accepts
either shape of grid evidence, stops reading inter-cell whitespace as rows, and stops reading a
stack of shaded lines as a grid; a grounding box's height is scaled by the rendered em rather than
the `Tf` operand; and `ground` withholds spans past `ethos.grounding.v1`'s cap instead of emitting
an artifact the verifier refuses.

**And the memory ceiling, byte-identically.** Four changes that move no emitted byte take the worst
gate document's peak from **6.49 GiB to 3.65**, every command that loads a representation from
about the payload's size less, and MCP `extract` on a 4.6 MB PDF from 8.7 GiB to 1.1.

### Added

- **`block` on `TextRunAttributes`** — which block of its page the leading-gap cut placed a run in,
  1-based in reading order. An **unnamed `Computed` index**, which is the only shape
  [`19-BLOCK-SUBDIVISION-SCOPE.md`](docs/19-BLOCK-SUBDIVISION-SCOPE.md) §6 permits without
  reversing P14: it says two runs are in different blocks and never that either is a paragraph.
  Absent wherever the rule declined, which — unlike `region` — is the ordinary case, because a page
  of uniform body text has no gap wide enough to open a second block and is supposed to have none.

  **Breaking in two ways a MINOR allows.** `TextRunAttributes` is not `#[non_exhaustive]`, so a
  downstream crate building one with a struct literal stops compiling. And it denies unknown
  fields, so a 0.54.0 reader — the library, its `ground`/`markdown`/`html`, or an SDK pinned to it —
  refuses a PDF representation from this build rather than ignoring `block`. Artifacts are already
  non-comparable across the version by `profile_sha256`; this makes them non-readable backwards too.

  The rule is a gap of at least **1.6 × the band's own modal leading**, in integers (`5·gap ≥
  8·leading`). Measured on the one gate document able to carry a real paragraph label:
  **63.7% of real paragraph breaks at 100% precision**, never firing mid-paragraph across 719
  chances. The third it misses is a ceiling rather than a shortfall — §9.2 measured that 35.1% of
  real breaks carry no extra leading for any gap rule to see. On `nist-sp-800-207` it produces a
  median of **9 blocks per page** and **0.12 blocks per line**, about eight lines to a block.

  `reading_order`'s existing `horizontal_cut` could not do this and its own doc says why: it cuts at
  *the widest gap and every gap tied with it*, and body text at uniform leading ties every baseline
  gap, so numbering its leaves would put one index on every line — *a line number wearing a block's
  name*. The criterion changed, not the machinery.

- **`READING_ORDER_RULE_V3`** — `gutter-columns-v3`, and **`TABLE_DETECTION_V4`**,
  **`TABLE_DETECTION_V5`** and **`TABLE_DETECTION_V6`** — the three ruled rule ids this release
  passed through, each kept as spelled because artifacts name it. Added to the public freeze and
  [`PUBLIC-API.md`](docs/PUBLIC-API.md).

- **`ci/release-artifacts.sh`** — builds and packages release binaries and labels each target in
  `SHA256SUMS.txt` **`verified`** (built, executed on the build host, every artifact digest over the
  gate corpus equal to the native build's) or **`compiled`** (built, never run). A target whose
  link step fails is reported rather than omitted. A script and not a workflow, because Actions
  cannot allocate a runner on this account and a `release.yml` would be a file that has never
  executed; a workflow can call it unchanged.

### Changed

- **`table_detection.ruled` `ruled-rects-v5` → `ruled-rects-v6`: `-v5` was wrong in both
  directions, and its benchmark could see only one.** Found while reviewing this release, before
  it was tagged.

  **It emitted stacks as grids.** The lattice is page-wide, so a column line can come from ink
  nowhere near the grid, and once empty bands are dropped a stack of full-width rectangles covers
  every face that remains. `nist-sp-800-207`'s disclaimer shades each line of one paragraph with its
  own rectangle and emitted as a **14 × 5 table whose thirteen cells each span all five columns** —
  its only interior column edges are the ends of a URL's 0.48pt underline. Every ruled table `-v5`
  emitted on the gate corpus was of that kind: that disclaimer on `207`, `218`, `161r1` and `53Ar5`,
  and two pairs of empty full-width bars on `171r3`. **Six fabricated tables in five of eight
  documents**, against a benchmark precision of 100%, because the benchmark's table documents are
  not NIST's. A grid must now be **divided inside itself on both axes**: some rectangle with an edge
  on an interior column line must span a kept row, and likewise for rows. That is the two-band floor
  asked of the ink instead of the band count, and like the floor it is no candidate rather than a
  refusal.

  **And it lost every grid drawn in rules.** A rule thinner than the lattice tolerance — pdfTeX
  draws `\hline` and `|` as exactly that — folds to one line and occupies no face, so selecting
  bands by faces kept none, and a grid `-v4` emitted by tracing produced **no table and no
  refusal**. On the benchmark that was a true 3 × 5 and a true 2 × 2. A rule now keeps the bands it
  crosses; a rectangle thick on both axes already occupied them, so no other page's selection moves.

  Measured with the new `docs/measurements/table-refusals/rule_ab.py`. Benchmark documents emitting a
  table go **12 → 14, every one holding a table in ground truth, zero false positives** — precision
  100%, recall 33%; ruled tables alone 10 → 12, `-v4`'s two back at `-v4`'s shapes. **On the gate
  corpus all six `-v5` tables are gone and none is added**, and over 41 further PDFs from the Ethos
  corpora it removes one more disclaimer and adds nothing. Every page `-v6` refuses is a page `-v4`
  refused. Compared byte for byte at equal profile, ten fixtures move: the six gate PDFs whose
  tables or refused pages change, and four — both IRS forms, `background-panel-not-a-grid` and
  `ruled-table-overlap` — whose tables and refused pages are identical and whose refusal only reads
  differently, below. No office fixture moves. **The refusal limitation also named `ruled-rects-v3`
  through both bumps of this rule** — the constant was spelled a second time in `limitations.rs` and
  nothing compared the two. It names the profile's rule now, and a test holds them together. Its
  explanation also said tracing *replaced* the face test, which it sits beside; it says so now. New
  public constant `TABLE_DETECTION_V6`.

- **`table_detection.ruled` `ruled-rects-v4` → `ruled-rects-v5`: the grid's own rows, not every
  band its edges imply.** `-v4` clustered every rectangle edge into lines and treated every band
  between them as a row or column. A table drawn as separated cell rows has whitespace between
  those rows, and that whitespace became a band nothing occupies — so the grid was larger than the
  page drew and the missing faces refused it.

  `01030000000045.pdf` paints **nine rectangles that are a complete 3 × 3 cell grid**. Its six y
  edges clustered into five bands, two of them inter-cell space: 5 × 3 = 15 faces with nine covered,
  which is the refusal's own arithmetic — *"9 rectangles implied 15 cells"*. **The rule declined a
  perfectly drawn grid over two rows it had invented.** It now emits `3 rows, 3 columns, 9 cells`.

  **A band no rectangle occupies is not a row.** Bands are selected before anything is asked of the
  grid, and both acceptance paths then speak of the rows and columns that exist — tracing
  especially, because a page must not be required to draw gaps it deliberately left.

  **And a grid needs two bands on both axes**, which is the face floor's own argument carried one
  step: two faces in a line is two boxes. Selection makes that shape reachable, since a page of
  framed form fields collapses to an N × 1. Without the floor the benchmark emits **17 documents
  with four false positives, every one single column**; with it, **12 with none**. It costs one true
  1 × 3, a lone header row geometry cannot tell from three boxes in a row. **So a single-row or
  single-column ruled grid 0.54.0 emitted is now no candidate at all** — no table, and no
  `ruled-table-candidate-refused` naming it, on the floor's own judgement that a stack of boxes is an
  ordinary page. On pages still refused, the detail's cell count is the kept grid's.

  Documents emitting a table go **7 → 12, all twelve with a table in ground truth, zero false
  positives** — precision 100%, recall 29%. **Zero false positives on the benchmark only:** on the
  gate corpus this rule emitted six tables and all six were fabricated — see `-v6` above, which
  removes them. Across this entry's two ruled changes: **5 → 12 documents, 2.4× the recall,
  fabrication at zero on the benchmark throughout.**
  `background-panel-not-a-grid` refuses at every step.

- **`table_detection.ruled` `ruled-rects-v3` → `ruled-rects-v4`: the ruled rule takes either of
  two shapes of evidence.** `-v3` had one precondition — every implied face covered by a rectangle
  that is not the enclosing border. That is right for a producer drawing cells and wrong for one
  drawing rules. Over `opendataloader-bench`, of the 42 documents whose ground truth holds a table,
  **all 30 that draw rectangles implying a grid were refused by that clause alone**, at a median
  57% of faces drawn: a page laying down row separators and no column separators has stated
  exactly where its grid lies while drawing almost none of its cells.

  `-v4` keeps that test and adds **line tracing** — every row and column boundary carried end to
  end by the rectangle edges lying on it, gaps closed by collinear ink only, merged rather than
  summed. **Either suffices.** Neither subsumes the other, which is why this is a widening and not
  a swap: a merged cell breaks an interior line, so tracing refuses what faces accept, and a
  rules-only grid draws no cell, so faces refuse what tracing accepts. Requiring both would keep
  all 30 refused. **Nothing `-v3` emitted is lost** by `-v4` — though the release as a whole gives up
  single-row and single-column grids, on purpose, at `-v5`'s two-band floor above.

  Documents emitting a table go **5 → 7**, all 7 with a table in ground truth, **zero false
  positives**. Tracing cannot fabricate: the lattice is built from rectangle edges, so a line
  nothing drew is not a lattice line. The enclosing rectangle counts toward the four outer lines,
  which it draws by definition, and helps no interior line —
  `background-panel-not-a-grid` still refuses under both paths. **The refusal reads differently on
  every page it names**: the kind `a cell the ink does not draw` becomes `a grid line the ink does
  not trace`, and each page's detail names the first line its ink does not carry.

  The gain was small, and the first diagnosis for it — one page-wide lattice traced across logos
  and borders — was wrong: T4 found the refused pages draw complete grids whose rows have whitespace
  between them, which `-v5` above addresses. Clustering rectangles into candidate grids is not the
  next change.

- **`reading_order_rule` `gutter-columns-v2` → `gutter-columns-v3`** on the PDF profile, so
  `profile_sha256` moves and every golden regenerates. A bump rather than a new name on
  `READING_ORDER_RULE_V2`'s own test: the rule reads the same evidence — whitespace in page space —
  and reports more of what it found. **One id and not two**, on that same doc's *"two ids for one
  rule would claim a precision that does not exist"*: one cut emits the order, the regions and the
  blocks, and none can move without the others. `-v2` keeps its spelling; artifacts exist under it
  and it promises a region and no block. **The nine office profiles are untouched.**

- **The unruled table rule tests its lattice-size cap before its gutter floor.** More than one
  precondition fails on a typical page and only the first is reported, so the order decides what a
  consumer is told. `MAX_FACES` is 4 096 and the median candidate is 4 628 faces on a page of prose
  and 6 672 on one holding a table, yet `LatticeTooLarge` fired **zero** times across 200
  opendataloader-bench documents because the gutter check always answered first with a few hundred
  centipoints of word spacing. Both were true; the word gap read as a near miss on a page whose
  candidate was a histogram of where words start. The reported refusal moves to
  `lattice_too_large` on **117** of 199 documents measured in isolation. The split moves again with
  every ruled change, because `unruled::detect` runs only on the runs no accepted ruled table
  already claims: on the tree this release ships it is **115 `lattice_too_large`, 81
  `gutter_below_floor` and 3 `faces_without_text`**. **No table changed at that commit** — 115
  tables across the eight gate documents, byte-identical — but the refusal is inside the artifact,
  so `extract`, `markdown` and `html` moved on every gate PDF, with their
  `representation_c14n_sha256`, while `table_detection.unruled` stayed `unruled-align-v1`: a rule
  id that reports a different first refusal for the same page.

- **The tagged role path is shared instead of cloned once per run and again per node.** Peak
  resident memory on the 733-page gate document falls from **6647.0 MiB to 4663.7 — 29.8%** — and
  the artifact is byte-identical, verified both by the goldens and by hashing both arms of an
  interleaved A/B across three repetitions. `bind_structure` deep-cloned the whole
  `PdfTaggedLocator` into every run and `to_representation` cloned it again into every node: ~21.9M
  `Vec<String>` elements per copy, held twice at peak, for **thirty distinct role paths over
  twenty-three distinct role names**, the dominant one 15 deep and carried by 1.2M runs. The tree's
  bindings hold `Arc<PdfTaggedLocator>` now, so 5,661 locators exist instead of 1.65M clones. The
  clone sites are unchanged — they were already `found.clone()` and are pointer clones now.
  −22.0% at `--max-pages 128`, −16.1% / −14.9% / −14.6% on the three mid-sized documents, −6.9% on
  a six-page form. Wall time falls 2.6–8.6% (interleaved, median of five), largest on the densest
  document; a 36% fall two sequential runs showed was machine state, and is not claimed.

  **`StructuralLocator::PdfTagged` now carries an `Arc`**, which is source-breaking for a
  downstream crate matching that variant. Thirteen in-tree sites are updated. By
  [`RELEASING.md`](docs/RELEASING.md) §4 the rule is output-based and byte-identical means PATCH, as
  0.37.1's "the run buffers move instead of cloning" was — but that rule measures bytes and says
  nothing about compilation. It ships inside 0.55.0, a MINOR on other grounds, so this version did
  not have to settle that; a PATCH that changed a public type's shape would.

- **MCP stopped parsing every artifact back into a tree.** `tool_extract` handed the canonical
  bytes to `serde_json::from_slice` so they could sit in a `json!` response, then serialized the
  response back into one String — the artifact three times over, the middle copy a value tree
  several times the text. A 4.6 MB PDF peaked at **8.7 GiB** over MCP against 1.4 GiB on the CLI,
  on the surface `mcp.rs` calls the one that matters most. It now peaks at **1.1 GiB (−87%)**, and
  MCP runs at about the CLI's own peak across the gate corpus. The 733-page document, never run
  through the old route because it would have needed ~30 GB, needs 3.7 GiB. **Byte-identical in the
  build that ships**: an artifact reply writes every key in sorted order around the canonical bytes,
  which is what a release build's `serde_json` printed — verified over 84 MCP sessions across
  `extract`, `ground` and `node_get`, and on the headline document, with zero differing bytes.
  **The envelope is now canonical in every build, which it was not:** core's dev-dependency
  `preserve_order` unifies into `cargo test --workspace`, so the old envelope was insertion-ordered
  in the gate's test build and sorted in release. The first version of this fix failed the gate on
  exactly that. `node_get` returns one node and keeps its `Value`.

- **A canonical object's largest field is adopted into the output instead of copied into it.**
  c14n sorts keys by staging every field in its own buffer and then copying each into the output,
  so the representation payload's `nodes` — 805 MiB on the largest gate document, 99.9% of the
  payload — existed twice during `seal`, which is where that document peaks. At the top level
  nothing has been written yet, so `CanonicalMap::finish` now builds the result around that buffer:
  shift it right in place, write the prefix into the gap, append the suffix. Peak RSS on
  `nist-sp-800-53Ar5` **4664.8 → 3717.0 MiB (−20%)**, footprint −910.9; −19% on `161r1`, −16% on
  `171r3`; byte-identical across 86 documents and every canonical subcommand; wall time within 2%. The worst gate document now peaks at 3.65 GiB, down
  from 6.49 GiB when this work began. **Reserving the output at its final size — the change first
  proposed — was measured beside it and bought nothing on any axis, and is not shipped.**

- **A representation's fingerprint is hashed as a stream.** `verify_fingerprint` rebuilt the
  payload's canonical bytes in full to hash them and threw them away — 805 MiB on the largest gate
  document — about the payload's size in memory for every command that loads a representation:
  `ground`, `markdown`, `html`, and MCP's `ground` and `node_get`. The payload now writes its eight
  members in the order c14n sorts them, serializing each value — each node, one at a time —
  straight into a sink that hashes 64 KiB at a time. A generic streaming sink would not have helped:
  c14n sorts an object's keys by staging every field, so `nodes` was materialized before a byte could
  reach a hasher. Measured against the build before it, interleaved, output byte-identical: `ground`
  on the largest document **4093 → 3299 MiB (−19%)**, `markdown` −766 MiB, `ground` −226 MiB on
  `161r1` and −70 MiB on `171r3`, with wall time within noise (−1.3% to +1.4%). It took three
  attempts: streaming each node through a scratch buffer saved the same memory but cost 4–7% of wall
  clock; hashing in 64 KiB blocks on its own changed nothing; serializing straight into the sink
  removed the copy and the cost with it. **It does not make verification faster** — the walk over
  every node is still most of a load's wall clock, and only hashing the input's own bytes would
  remove that. **Byte-identical by construction and by proof:** `seal` still hashes the materialized
  bytes it keeps for the emit path, so the two routes must agree or every sealed artifact would fail
  its own verification, and a test states it outright. A PATCH: no emitted byte moves.

- **The PDF reader keeps each glyph's advance instead of summing them away.**
  `ShownText::code_advances` holds the per-code deltas `show_text` used to add into one total and
  discard, which is what a sub-run extent — a word box — has to be built from. Aligned with the
  codes, not the characters, because a code may decode to more than one; `scalar_code_mismatch`
  flags exactly that, and fires zero times over the four gate fixtures' 155 000 nodes, so anything
  indexing these by character cannot prove it is wrong on this corpus. Crate-internal and on no
  wire: a `debug_assert` where the run is consumed checks one entry per code and that the entries
  account for the total. `ci/artifact-bytes.py --small` is byte-identical over 236 artifacts.

### Fixed

- **The ink box was scaled by the raw `Tf` operand rather than the rendered em.** On a page that
  carries its type size in the text matrix, every grounding box came out about a point tall — all
  **82 909** of them on `nist-sp-800-207`, whose body text is 13.3pt. The *width* was already
  carried through the CTM, so the two axes of one rectangle were in different spaces, which is why
  adding only the text matrix would still have been wrong. Scaling is now by the vertical component
  of the text rendering matrix (§9.4.4), so a size carried in the text matrix or the CTM measures
  as one carried in the operand does. No test caught this: geometry sits outside
  `representation_c14n_sha256`, and the one height test exercises only the operand-carried path.

  On that document the 82 909 measured run boxes go from 110–123 centipoints tall to **665–3 103,
  median 1 329**, none at or below 2pt. Every measured box in `extract`, and every `ground` span
  and element box built from one, moves — **while `representation_c14n_sha256` does not**, so a
  consumer comparing that fingerprint will not see this change; `profile_sha256` is the one that
  moves. The wire `font_size` still carries the raw `Tf` operand, deliberately.

- **Three files claimed 0.50.0 against a 0.54.0 tree** — `README.md`, `docs/README.md` and
  `docs/CAPABILITY.md`, the last of which opens with a rule requiring it to move with the version.
  `ci/doc-version.sh` now checks every stated version against the workspace in both `ci/gate.sh`
  and CI — those three and `packages/node/package.json`, which cannot move alone (its vendor digests
  and docushell's pin move with it). A claim whose pattern stops matching fails as a blind check
  instead of passing. `packages/python/pyproject.toml` is exempt: its version is dynamic.

- **`ground` emitted an artifact the Ethos verifier refused for large documents.**
  `ethos.grounding.v1` caps `spans` at a million, and the projection never looked: the 733-page gate
  document produced 1,619,510 spans, the engine's own `grounding-check` called the artifact
  `invalid` and `ethos grounding check` refused it (`limit_exceeded` at `/`), while `ground` exited
  0. Past the cap the projection now keeps every element and withholds the spans — all of them,
  never truncated to the cap, since a partial set would ground some runs and silently drop the
  rest — and declares it three ways: `capabilities.spans: false` in the artifact, a stderr note
  naming `spans-withheld-over-schema-limit`, and MCP's `ground` summary; library callers read
  `Projection.spans_withheld`. Verified with release builds: on the 733-page document
  both checkers now accept the artifact — the engine's `grounding-check` and `ethos grounding
  check` each exit 0, where they returned 1 and 2 — and it carries its 50,329 elements, no spans,
  and shrinks from 151.4 MiB to 6.6 MiB; `ground` on the three next-largest gate documents is
  byte-identical to before. Every artifact under the cap is byte-identical, because the
  rule only engages past it. **An emitter change, so a MINOR** — although the only outputs that move
  are ones no verifier accepted. New public items `SpansWithheld` and `SPANS_WITHHELD_OVER_LIMIT`,
  frozen and documented. The end-to-end test grounds the 733-page document through both checkers
  and is `#[ignore]`d, because through the debug binary it would add minutes to every gate run; the
  rule itself is tested in the grounding crate with a small cap. **Not covered:** the schema's
  other limits — 5,000 pages, a million elements, 100,000 tables, 16 KiB strings — are still not
  enforced by the projection. Nothing in the corpus comes near them.

### Measurements

- **`docs/measurements/cross-architecture/`** — output does not depend on the instruction set.
  An `x86_64-apple-darwin` build from the pinned 1.88.0, executed under Rosetta 2, matches native
  `aarch64-apple-darwin` on **412 comparisons over 86 documents with zero mismatches**: `extract`
  and `classify` from a PDF, `markdown`, `html` and `ground` from a representation, and 22 refusals
  compared by their stderr. It is the axis integer centipoints exist to settle, and it had never
  been tested. The operating-system axis has not been: Linux and Windows remain unbuilt and
  unexecuted. The instrument's first run was an overclaim — 344 comparisons, 194 of them `markdown`
  and `html` refusing a PDF they were handed — and was corrected before this figure.

- **`docs/measurements/omnidocbench/`** — "684 of the 981 pages carry a text layer, the other 237
  score 1.0" did not reconcile: 684 + 237 is 921, the pages the harness scores, not the corpus. Three
  populations were sharing one denominator — 981 in the corpus, 921 scored, 756 carrying text
  objects — and each is now named where it is used.

- **`docs/measurements/table-refusals/`** — six studies of why this engine finds so few tables,
  which drove the three ruled changes above.

  **T1–T1b, against 0.54.0:** the engine emitted a table on 5 of the 42 benchmark documents that
  hold one, all five from the ruled rule; the unruled rule refused on 199 of 200, every refusal
  `gutter_below_floor`. Table and prose pages share a 416-centipoint median gutter only because
  `gutter_fault` reports the leftmost sub-floor gap, which on any prose page is word spacing — so
  `COLUMN_GUTTER_MIN` is not the lever. Crossed with ground truth, no lattice metric separates a
  table page from a prose page, and three of five are inverted.

  **T2:** scoping the unruled candidate to a block, as T1b predicted, shrinks the lattice 30–40× and
  still emits nothing — zero candidates would emit at either scope. The fold tolerance and the
  gutter floor contradict each other, and occupancy sits at a median 54% on table and prose blocks
  alike. **T3:** a perfect unruled rule could address at most 12 of the 42, at a measured ceiling of
  17% precision, since prose blocks outnumber table blocks 10.6 to 1; 30 of the 42 draw rectangles
  the ruled rule refused on one clause, so the rework went there. **T4:** those refusals were not a
  page-wide lattice crossing logos, as `-v4` first diagnosed, but whitespace rows the lattice
  invented. **T5** is `-v5`; **T6** is `-v6`, and adds `rule_ab.py`.

- **`docs/measurements/block-subdivision/probe3b.py`** — the instrument behind
  [`19`](docs/19-BLOCK-SUBDIVISION-SCOPE.md) §11.2's numbers, which had never been committed. Its
  headline reproduces eight releases later (63.7% against 63.0%), and 1.15×'s published 1.1%
  false-fire has gone to zero, so the margin that chose 1.60× is now zero. §11.4 forbids acting on a
  one-document result, so 1.60× ships — but a second labellable document would now decide a live
  question.

- **`docs/measurements/memory-ceiling/`** — this file's own memory figure was wrong, and is
  withdrawn. `ci/bench.py` shipped "peak RSS runs roughly **300x** the input"; measured over the
  eight gate documents it runs **143x to 933x**, a 6.5x spread, and on the engine that ships 135x
  to 524x. It is a corpus median and never was a bound. "~4.7 MiB per page" was also a median — the
  real range was 3.20 to 9.07, now 3.01 to 6.58.

  Three things nobody had measured. The corpus's worst case had only ever been extrapolated: it is
  **6.5 GiB for a 7.12 MiB input**, now 3.65. Peak is **linear in admitted pages within one
  document**, so there is no superlinear retention — the cross-document spread is content density.
  And **`--max-pages` leaves a floor it cannot lower**: `--max-pages 0` costs 222 MiB on the
  733-page document, because `structure::read` and `tree_mcids_by_page` are built above the
  `let budget`. So the two ceilings in the tree do not compose into a memory bound — nothing maps a
  permitted 2 GiB input onto a peak figure, and a caller who needs a hard memory ceiling still does
  not have one. `extract --help` gives a sizing rule instead of a ratio: about 7 MiB per admitted
  page plus 0.35 MiB per page in the document, which over-predicts the worst gate document by 44% —
  loose, in the safe direction.

  Four proposals died with a measurement rather than an argument, recorded so the slice is not
  spent twice: streaming the artifact buffer (950 MiB at an instant that sits 1.1–2.1 GiB below the
  high-water mark), mimalloc (**+22%**), bounding rayon's threads (~97 MiB, +47–54% wall clock),
  and narrowing the page fold, which is v2-S15's refusal in disguise.

- **Freeing memory sooner was measured and refused: it raised the peak.** An audit claimed that
  dropping the lopdf object graph before `to_representation` saved 838 MiB; its refuter put the
  whole graph at 149 MiB. Rebuilt and measured against today's engine, interleaved, five runs per
  arm, every artifact byte-identical: it **raises** peak RSS by 343 MiB and peak *footprint* by
  **668 MiB** on `nist-sp-800-53Ar5`, and by 211 / 241 MiB on `nist-sp-800-37r2`. Dropping the
  whole extract page graph before `seal` as well adds nothing. Footprint rising more than RSS rules
  out reclaimable pages: the change genuinely raises what macOS charges the process. The auditor's
  −838 MiB was a real measurement of the engine before role-path sharing; the sign flipped with an
  unrelated commit, which is the reason to refuse it rather than re-tune it. The lesson recorded in
  `docs/measurements/memory-ceiling/` §9: in this engine, memory is released by not allocating,
  not by freeing sooner. Baseline footprint also runs 13–22% below RSS, so the RSS-based sizing
  rule over-provisions on macOS — the safe direction.

- **The read side had never been measured, and verification is most of it.** `ground`, `markdown`,
  `html` and MCP's `ground` and `node_get` load a representation and verify its fingerprint by
  rebuilding the whole canonical payload. On the largest gate document `ground` peaks at 4.17 GiB —
  **531 MiB above `extract` producing the same artifact** — and measurement-only arms, never
  shipped, show verification accounts for about the payload's size in memory (805 MiB there) and
  **55–64% of the wall time on every read command at every size**: `ground` on that document runs
  18.4 s with it and 8.3 s without. Freeing the source buffer early does nothing. A streaming hash
  would not help, because c14n stages `nodes` in full to sort keys; hashing the input's own bytes
  would, but is not verdict-identical — about 25 fields parse an explicit empty value the same as
  its absence — so the fix is left as an owner's choice among three options. *The memory half has
  since shipped — "A representation's fingerprint is hashed as a stream", under Changed, found a
  route around the staging this sentence describes: `ground` on the largest document now peaks at
  3299 MiB, below `extract`'s 3717, so the 531 MiB headline no longer holds. The wall-time half has
  not.* `docs/measurements/memory-ceiling/` §12. `grounding-check` peaks at 12.7–13.5x its input,
  the worst ratio in the engine, from parsing its artifact twice. **Found on the way, and fixed in
  this release** (see Fixed): `ground` emits a grounding artifact the Ethos verifier refuses for the
  largest gate document — 1,619,510 spans against `ethos.grounding.v1`'s 1,000,000 cap, which the
  projection does not enforce — so grounding fails silently for documents past roughly 450–750 pages
  while `ground` exits 0. Confirmed by the engine's own check, an exact count, and `ethos grounding
  check`.

---

## [0.54.0] — a word gap the page opened by moving the cursor

**A PDF may open a word gap without drawing a space glyph** — it moves the text cursor with a `TJ`
adjustment instead. `content.rs` already recognises that, writes the space into the run's own text
and flags it `synthesized` so nothing mistakes it for a character the document contains.

**The block rule then measured the same gap a second time** — against `INK_EPSILON_CENTIPOINTS`,
a 12-centipoint *quantization* epsilon — decided it was a break, and started a new block. So a
page whose word spaces are cursor moves emitted **one word per block**:

```
coordination          coordination geomet
geomet          →     ry with a N
ry
with
a
```

**Measured over the 981 OmniDocBench documents**, before and after, per document:

| | 0.53.0 | 0.54.0 |
| --- | ---: | ---: |
| documents whose block count changed | — | **296 of 735 (40%)** |
| documents made **worse** | — | **0** |
| total blocks | 463 715 | **405 052** (−12.7%) |
| median characters per block | 5.42 | **8.40** |

The largest single change is 1 774 → 356 blocks, 5.3 → 30.4 characters each.

**It introduces no threshold, and that is the point.** A gap epsilon sized to word spaces was
measured and refused at 0.47.0 — *"Latin has a trough to site it in and CJK has none, so it would
be a measurement on one script and a tuned knob on the other."* That was re-measured here over
**749 409 same-baseline pairs**: for Latin pairs carrying a synthesized space the distribution
decays monotonically from its word-space mode at 0.6–0.7 pitch, and the shallowest band is 419 per
unit against 929 below and 494 above — a factor of 1.2, not the empty valley `INK_EPSILON`'s own
justification rests on. **There is no trough to site a ceiling in, so this rule has no ceiling.**

What stands in for one is that **the space is already in the run's text either way**. Joining moves
a block boundary; it changes no byte of text. And the *declared* path has always joined on exactly
this test — `drew_space`, with no gap check at all — because an mcid group is the document's own
statement that two runs belong together. This is that clause reaching the undeclared path, where
the reader's own insertion is the only statement available.

**No golden changed, for the second release running.** `SynthesisReason::TjGap` fires zero times on
four of the five gate documents, so the fixtures cannot see this rule at all.
`a_cursor_moved_word_gap_does_not_break_the_line` now holds it, verified by reverting the change
and watching it **fail**.

**What this does not fix.** Intra-word splits survive where a run's advance exceeds even the
widened `ink_reach` cap — `geomet` / `ry` in the sample above, where the advance runs 17% over the
font median. And CJK is untouched by construction: it draws no word spaces, so no space is
synthesized and no join is offered. The 0.47.0 refusal stands there in full.

`markdown_rule` moves `markdown-blocks-v6` → `-v7` and `html_rule` `html-blocks-v6` → `-v7`,
together, because the change is in `ink_sequenced`, which both projections and `geometric_blocks`
call.

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
