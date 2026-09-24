# 29 — Outlines scope: the hierarchy the author wrote, emitted as the author wrote it

**Status: proposal. Nothing here ships until the owner reopens `17-D1-SCOPE.md` §5's evidence bar**
— §2 is the argument, §2.1 the measurement it rests on. Written 2026-09-23 against `main` at
`3bc9126`, workspace 0.60.0.

Source: [`06-STEAL-REFUSE.md`](06-STEAL-REFUSE.md) row **PI-A**, from the PageIndex review. **Every
number below was measured by this session** with [`measurements/outlines/`](measurements/outlines/README.md).
A title-versus-page-text rate briefly published in PI-A was **withdrawn as unmeasured** (`3bc9126`);
that join is §7's and has not been run.

**Revised after an adversarial review** (four lenses: doctrine, code, measurement, omissions). It
found two fatal defects in the first draft — the withdrawn number, and an inverted encoding
precedent that put a refused dependency on the critical path. §4 and §9 are rewritten because of the
second, and the ordering they now carry is the opposite of the draft's.

---

## 1. The one sentence

**A PDF's `/Outlines` tree is a hierarchy the author wrote down, and this engine has never opened
it** — so it is emitted as its own record, with the author's depth, the author's title bytes, and a
counted absence wherever the document's own destination, page or encoding does not resolve.

## 2. What must be reopened first, and it is not P14

A repo-wide search of `crates/` for `Outlines`, `Bookmark` and `/Dest` returns **zero**;
[`structure.rs`](../crates/ethos-parser-pdf/src/structure.rs):270-275 asks the catalog for
`/StructTreeRoot` and nothing else.

**P14 does not bear on this.** It refuses *role inferred from presentation*. A `/First`/`/Next`
chain is neither inferred nor presentation — it is a declaration, in the file, by the author, the
same class as `/RoleMap` and the tagged tree. That is why this sits beside **O13** and **P19** and on
`structure.rs`'s own rule, *"Consume, never synthesise."*

**`17-D1-SCOPE.md` is what bears on it.** §3 refused `/Outlines` as a **document-boundary** signal —
*"a three-hundred-page book with chapter bookmarks is one document"* — which is right and is not what
this proposes. §6's fourth bullet is the sentence that looks fatal and is quoted here in full rather
than trimmed:

> **Not a reason to read `/Outlines` "while we are here".** … Putting one on the wire **under a name
> suggesting a document boundary** would be P14 wearing a different hat — structure inferred from
> presentation, indistinguishable on the wire from structure the author declared.

Both halves are conditioned: *"while we are here"* scopes the bullet to D1's own investigation, and
the harm named is a **boundary-shaped name**. A scope document with its own measurement is not
"while we are here", and an outline emitted as an outline is not a boundary. What must be re-argued
is §5's standard — *a detector with no positive case is an assertion* — which §2.1 meets.

### 2.1 The measurement §5 asks for

Measured 2026-09-23 over all **70** fixture PDFs with
[`outlinescan.rs`](measurements/outlines/outlinescan.rs) (`lopdf` 0.44.0, the engine's own pin).

| | |
| --- | ---: |
| PDFs carrying an `/Outlines` with at least one entry | **6 of 70** |
| total entries | **2 273** |
| maximum declared depth | **5** |
| destinations resolving to a page in this document | **2 273** |
| destinations **not** resolving | **0** |
| entries whose target page precedes the previous entry's | **2** |
| empty titles | **0** |
| **walks abandoned early** (repeated id, or unresolvable `/First`/`/Next`) | **0** |

| document | entries | depth | pages |
| --- | ---: | ---: | ---: |
| `nist-sp-800-53Ar5` | 1 251 | 5 | 733 |
| `nist-sp-800-161r1` | 433 | 5 | 327 |
| `nist-sp-800-37r2` | 347 | 3 | 183 |
| `nist-sp-800-171r3` | 160 | 3 | 120 |
| `nist-sp-800-207` | 69 | 3 | 59 |
| `nist-sp-800-218` | 13 | 2 | 36 |

**2 273 is exact, not a lower bound.** The walk counts its own early exits and they are zero, and
the total was confirmed a second time by an independent route — the count of objects in each file
carrying both `/Title` and `/Parent`, which is 2 273 as well, from qpdf's object dump rather than
lopdf's tree walk.

`17-D1-SCOPE.md`:54 already recorded *"8, in six documents"*, so the **six** is not new. The 2 273,
the resolution rate and the zero truncations are.

## 3. What is read

One record per entry, and the record carries **one `derivation`**, on
[`tables.rs`](../crates/ethos-parser-core/src/tables.rs):337's precedent — *"whose statement the grid
is"* — rather than one class per field:

| | |
| --- | --- |
| the record's `derivation` | **`Extracted`** — the hierarchy and the titles are the author's statement, read off `/First`/`/Next` |
| the `/Title` string's decoded text | present only when it decodes; otherwise **absent and counted** (§4) |
| the declared depth | the `/First`/`/Next` chain's, **never renumbered** (§6) |
| the outline item's object id, **both halves** (`object`, `generation`) | on `PdfObjectLocator`'s rule that *"an object id is both halves"* |
| the 1-based page the destination resolves to | a typed presence/absence, `Computed`, under the rule id below |

**A versioned rule id, `outline_rule: outlines-v1`**, on the profile beside `struct_tree_rule` and
its siblings, `NOT_RUN` on the eight office profiles. It covers the walk order (`/First` then
`/Next`, pre-order), the depth semantics, and the destination-resolution list — so §8 bar 1 has a
version to move when any of them changes.

Destinations resolve per §12.3.2: an explicit array, a name or byte string through `/Names`→`/Dests`
or the catalog's older `/Dests`, and `/A` with an `/S /GoTo` action. An integer first element is a
**remote** destination's page in another file and is not resolved. **S2 adds this engine's first
`/Names` name-tree reader and its first object-id-to-page-index map; neither is reuse.**

## 4. Titles: what is decodable, and why decoding is not a precondition

`decode_text` ([`forms.rs`](../crates/ethos-parser-pdf/src/forms.rs):490) already implements §7.9.2.2
— UTF-16BE behind a byte-order mark, PDFDocEncoding otherwise. **Reuse costs a visibility widening**:
it has no modifier inside `pub(crate) mod forms`, so a sibling module cannot call it.

**Its non-UTF-16 branch is `bytes.iter().map(char::from)` — Latin-1.** Byte `0x85` becomes
**U+0085**, a C1 control character. Measured over the 2 273 titles:

| byte | occurrences | what Annex D.2 assigns | what this repository's only 0x80–0x9F table would give |
| --- | ---: | --- | --- |
| `0x85` | 59 | **U+2013** en dash | U+2026 ellipsis ❌ |
| `0x84` | 9 | **U+2014** em dash | U+201E ❌ |
| `0x90` | 2 | **U+2019** right quote | **undefined** ❌ |

**69 titles carry those 70 occurrences** — one title in `nist-sp-800-171r3` carries two `0x85`. The
Annex D.2 column was read with qpdf 12.3.2's decoder and agrees with the strings' own context:
`03.08.09 System Backup – Cryptographic Protection`, `PREPARE TASKS AND OUTCOMES—SYSTEM LEVEL`,
`EXECUTIVE ORDER 14028's CALL`.

**The wrong table is one file away and would pass a printability check.**
[`encoding.rs`](../crates/ethos-parser-pdf/src/encoding.rs):302-329's `WIN_ANSI` is Windows-1252; it
turns `Backup – Cryptographic` into `Backup … Cryptographic`, and has no entry for `0x90` at all. The
first draft of this scope made exactly that error.

### 4.1 Why the draft's ordering was wrong

The draft made vendoring Annex D.2's block a **precondition** and called `WIN_ANSI` *"generated"*. It
is not: `build_win_ansi` (`encoding.rs`:203) is ~220 hand-typed lines under the heading *"Windows-1252,
written out"*, and the generated file is `winansi_names.rs`, derived **from** it and emitting only
entries **three independent sources agree on**.

That matters because [`21-STANDARD-14-ASCII-COVERAGE-SCOPE.md`](21-STANDARD-14-ASCII-COVERAGE-SCOPE.md)
**§4 refused a hand-transcribed Annex D table on measurement**, and §5's reopening condition is *"a
derived table rather than a transcribed one … cross-validated against the `WinAnsiEncoding` text
column already vendored"*. **That condition cannot be met for this block**, because 0x80–0x9F is
precisely where the two tables disagree — cross-validating against WinAnsi here would enforce the
wrong answer.

**So decoding is not on the critical path.** The record ships with the titles that decode, and the
ones that do not are **counted under a named code and left absent** — the engine's ordinary answer
to *I could not read this*, needing no new table and crossing no refusal. On this corpus that is 69
of 2 273 (3.0%); a consumer is told, and no title is guessed.

Vendoring the block becomes a **separate, optional** improvement (§9 `S-ENC`) that must clear
`docs/21` §5 on its own terms. Its argument is different from the one §4 refused — 32 entries rather
than 224, three of which occur here, with qpdf's decoder and the strings' own context as the two
independent confirmations — but it is **the owner's to take, and nothing in §8 depends on it.**

**A defect found in passing.** `decode_text`'s doc comment says *"a byte outside it becomes
`U+FFFD`"*. Its non-UTF-16 branch cannot produce `U+FFFD`. True of the UTF-16 branch
(`from_utf16_lossy`), false of the other. Correct it on its own — **not** by making the comment match
the code, because the comment describes the better behaviour.

## 5. Where it goes on the wire, and what that costs

**Its own record on `RepresentationPayload`, beside `tables`. Never a `Node`.**

North Star #4 requires a native locator on every node, and a bookmark title is **text no content
stream painted** — no page, no origin, no advance, because the document wrote it into the catalog
rather than drawing it.

**`NativeLocator::PdfObject` already exists** ([`representation.rs`](../crates/ethos-parser-core/src/representation.rs):153,
added at v1-S4 for annotations and form fields, which *"are not glyph runs"*). It is the closest
shape and is still rejected: its `page` is a **required** `u32`, and §3's counted case is an entry
whose page does not resolve. The record therefore carries no `NativeLocator` at all, on
`TableRecord`'s precedent.

**The consequence, stated rather than discovered: an outline title is not quotable.** `locate`
searches `geometric_blocks(&payload.nodes, …)` and grounding builds elements from `payload.nodes`, so
a field outside `nodes` is reached by neither. It is evidence about declared structure, not about text.

**A ninth field is a schema event, and the draft named none of it.** All of the following move in S2:

- **`write_canonical` destructures all eight members**, and `representation.rs`:2155 says why: *"The
  destructuring is the guard. A ninth field fails to compile here rather than being hashed by
  `canonical_bytes` and silently skipped."* Adding the field is a compile error until the writer is
  updated — which is the design working.
- **`REPRESENTATION_SCHEMA_VERSION`** (`representation.rs`:78) bumps `0.6.0` → `0.7.0`, with the same
  non-comparability statement its doc comment already makes for the two prior bumps, plus its guard
  in `contract_invariants.rs`.
- **`#[serde(deny_unknown_fields)]`** means the new field needs `#[serde(default)]`, and the bump is
  breaking for any pinned consumer.
- **`document-representation.draft.json`** and the pinned worked examples.
- The **capability guards** at `capabilities.rs`:352 and `assurance.rs`:524.

## 6. Refusals

Three are settled in [`06-STEAL-REFUSE.md`](06-STEAL-REFUSE.md), one in
[`17-D1-SCOPE.md`](17-D1-SCOPE.md), and two are this scope's own.

1. **No title repair (PI1).** A node's text is never overwritten from a navigation label. Contract §6
   rule 1: *"Nothing overwrites `Extracted`."*
2. **No fabricated entries (PI8).** No `Page N`, no `Preface`, no title this engine chose.
3. **No renumbered depth** (PI-A's own cell). PageIndex re-stacks levels after dropping entries, so
   its wire depth is a position in a pruned stack. The depth here is the chain's, unchanged, even
   where it skips a level.
4. **No section end** (`17-D1-SCOPE.md`:130 — *"a boundary inferred from a bookmark is invented no
   matter how reasonable the inference looks"*). §2.1's **two backward-stepping entries** are the
   concrete reason: there the next entry's page is *behind* this one, so an inferred end is negative.
5. **No entry dropped.** An unresolved destination, an undecodable title and a page outside the
   budget are each **absent and counted**, never silently omitted — PageIndex drops all three.
6. **No binding to a heading node on the target page.** The entry's page is the furthest the
   document's own words reach; matching it to a run would be inference dressed as declaration — the
   same shape PI11 refuses for captions.

And one hazard: a **cyclic or self-nesting** outline is refused under its own name, on the fail-closed
precedent `structure.rs` already sets for a cycling `/K`. The instrument measured 0 on this corpus,
which is why this is a refusal rather than a bar.

## 7. The cross-check is a count, not a filter (PI-B)

`embedded_toc.py:286-295` asks whether a bookmark's title appears in the text of its resolved page —
**E6's shape pointed at a new pair**. PageIndex spends it as a silent insert gate.

Here it is **one counted code**: an entry whose title was not found in the text of its resolved page.
No entry dropped, no title rewritten. The folding it needs is its own versioned rule id, not a flag
— `locate-scalar-exact-v1`'s precedent.

**Its own slice and its own measurement, and it is gated on §4**: an undecoded `0x85` cannot match a
page drawing an en dash, so the count would measure the decoder rather than the document. **This is
why PI-A's 97.4% was withdrawn rather than re-derived** — the number could not have meant what it said.

## 8. The acceptance bar, set before the code

1. **Every entry resolves, at the numbers already measured.** On the six fixtures the engine's
   resolver returns `entries == resolved` with `unresolved == 0` and `truncated == 0`, per-document
   1 251 / 433 / 347 / 160 / 69 / 13 at depths 5 / 5 / 3 / 3 / 3 / 2. **Any shortfall against those
   constants fails**, and an entry that does not resolve is counted under a named code rather than
   dropped. (The draft's version was a disjunction every outcome satisfied.)
2. **No document without an `/Outlines` changes beyond a named carve-out**: its `profile_sha256`,
   `schema_version`, the empty outline record, the `outline-absent` declaration and the artifact
   fingerprint those move. **Same `nodes`, same Markdown, same HTML, same grounding ordinals and
   offsets, byte for byte.** 64 of the 70 fixtures are that population. *(The draft said "changes at
   all", which bar 4 contradicts: limitations live inside the payload and are hashed.)*
3. **The 69 titles decode to the exact scalars, or are absent and counted.** If `S-ENC` ships:
   `0x85`→U+2013 ×59, `0x84`→U+2014 ×9, `0x90`→U+2019 ×2, and **no title holds a scalar in
   U+0080–U+009F**. If it does not: those 69 titles are absent, counted, and the count is 69.
   *(Printability is not the test — `WIN_ANSI` passes it and is wrong.)*
4. **Two declarations, not one.** A `Capabilities` flag carries *this profile did not look* — false
   on the eight office profiles, with its partnering entry in `declared_limitations`. A document-scoped
   `outline-absent` carries *this document's catalog names no outline*, on
   `untagged_structure_tree_absent`'s wording (*"This document's catalog declares no
   `/StructTreeRoot`"*), which is a statement about the **document**. *(The draft conflated them.)*
5. **A budgeted page is a typed absence, not a bare integer.** Where `PageBudget` does not admit the
   target page there is no `PageRecord` to point at (`extract.rs`:1376, 1393-1402). The entry is
   emitted with its page absent and counted beside `resource-limit-pages`.
6. **`tag`'s round trip preserves the record**, entry for entry including object id and generation.
   All six outline-carrying fixtures are tagged and `tagging.rs`:2408 refuses a document that declares
   `/StructTreeRoot`, so **this bar needs a new fixture: untagged, carrying an `/Outlines`**. Without
   it the two features are disjoint on this corpus and the bar is untestable — say so rather than
   letting it pass vacuously.

**What would re-refuse it:** an inability to resolve destinations `lopdf` resolves, or any need to
guess a byte, a page or a depth to fill the record.

## 9. Slices

| # | What | Gate |
| --- | --- | --- |
| **S1** | The `outlines-v1` reader: walk, resolve, emit the record; titles that decode carry text, the rest are absent and counted; `outline-absent` and the capability flag; the schema bump and everything in §5 | §8 bars 1, 2, 4, 5 |
| **S2** | The untagged-with-outline fixture, and `tag`'s round-trip comparison | §8 bar 6 |
| **S-ENC** | *Optional, owner's.* Vendor Annex D.2's 0x80–0x9F, clearing `docs/21` §5 on its own terms; `decode_text` uses it; its doc-comment defect corrected | §8 bar 3's first half |
| **S3** | PI-B's cross-check as a counted code with its own rule id | Its own measurement, after `S-ENC` |

**`S-ENC` is no longer first and no longer required.** It is independently useful — `decode_text` is
live today on annotation and form-field strings, so the block would fix those too, and **how many of
those strings carry a byte in 0x80–0x9F across the 70 fixtures is not measured and should be before
`S-ENC` is argued.**

## 10. What this scope does not decide

Whether it is built. §2 argues for reopening `17-D1-SCOPE.md` §5's bar; the decision is the owner's.
It is filed as a row in [`OPEN-WORK.md`](OPEN-WORK.md) §4 in the same commit as this document — the
first draft asserted that row existed before it did.
