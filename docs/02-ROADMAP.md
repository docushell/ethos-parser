# 02 — Roadmap

One page, deliberately. Nine versions, one line each. No new version numbers get invented — every
change from research folds into a row that already exists.

Five versions have full scope and milestone documents: v0, v1, v1.1, v1.2 and v2, and **v2.2 has a
scope document for its first half** (decision #19) **and, since 2026-09-16, a scope and a milestones
document for its second half** ([`23-AUTO-TAGGING-SCOPE.md`](23-AUTO-TAGGING-SCOPE.md),
[`24-AUTO-TAGGING-MILESTONES.md`](24-AUTO-TAGGING-MILESTONES.md)). **v3 and v4 stay one line each**,
which is what "one page" protects.

---

## Versions

| Ver | Theme | Contents | Gate |
| --- | --- | --- | --- |
| **v0** | Honest PDF core | Classify · position-aware text runs · measured ink box or typed absence · synthesized flags · single-column order · format detection · error taxonomy · canonical JSON · capability declarations · grounding emit and check · CLI and library · fuzz and mutation tests | The validator agrees byte-identically with `ethos grounding check` on all 15 fixtures |
| **v0.1** | Verify and robustness | Shell out to the Ethos CLI as a declared capability · encoding-issue detection · the bounded xref repair | An ungrounded claim exits 1 with a report, and nothing is silently skipped. **Met** |
| **v1** | **The DocuShell replacement gate** | Tables, ruled and unruled, with a locator cross-check · vector paths driving ruled detection · full element vocabulary · multi-column reading order · tagged-PDF structure trees · forms and annotations as distinguishable nodes · images · security findings · annotated overlay | Fabrication **0** and an honest table number on the set this repo owns. Measured at 70‰ macro over twelve documents against the parked 0.489 comparator. **Closed** by decision #18 (2026-08-30) on a capability statement and a band, not on a macro — see [`00-NORTH-STAR.md`](00-NORTH-STAR.md) row 18 |
| **v1.1** | Safe Markdown | Markdown and HTML, each with an anchor map · hyphenation and similar cosmetics as export-only | A Markdown-quoted citation verifies end to end |
| **v1.2** | Adoption | MCP server · Python and Node SDKs · LangChain tools | Locators survive every adapter round trip |
| **v2** | Office formats | DOCX, XLSX, PPTX, ODT, ODS, ODP, RTF, EPUB · CSV argued and refused · one shared record and one serializer · embedded assets counted | A DOCX quote and an XLSX cell both **bind** to an address the file states, with **no synthesised pages**. **Met** |
| **v2.2** | Layout and accessibility | **Geometric block structure** — the reading-order cut's own column bands, emitted per node as `Computed` layout · **the block cut, shipped 0.55.0** as `TextRunAttributes.block` under `gutter-columns-v3` — the unnamed `Computed` index [`19-BLOCK-SUBDIVISION-SCOPE.md`](19-BLOCK-SUBDIVISION-SCOPE.md) §6 permits, and shipped **without the scope document this page requires before code**; its limits ride on every PDF artifact as `block-subdivision-leading-gap-only` · **auto-tagging, built on the branch for the next version**: the reader binds an engine-written `/Div` as `Computed` under `struct-tree-v2`, and the writer, the `tag` subcommand, fills absence only — scope [`23-AUTO-TAGGING-SCOPE.md`](23-AUTO-TAGGING-SCOPE.md), slices [`24-AUTO-TAGGING-MILESTONES.md`](24-AUTO-TAGGING-MILESTONES.md) | **Reopened by decision #23.** Clause one — a region is emitted wherever the cut divided a page and nowhere else — **met** at 0.42.0. Clause two was closed by #21, because a `/P` written from a `Computed` cut reads back as `Extracted` and is uncorrectable; #23 answers that with an attribute object under `/A` carrying the derivation, a shape `structure.rs` already parses. **Clause two is built on the branch for the next version** (2026-09-17): the reader binds an engine-written `/Div` as `Computed` (`struct-tree-v2`), the writer `tag` fills absence only, and the block cut is its consumer. It is **met on the fixtures once [`24-AUTO-TAGGING-MILESTONES.md`](24-AUTO-TAGGING-MILESTONES.md) S3's round-trip tests pass**, measured in [`measurements/auto-tagging/`](measurements/auto-tagging/); the owner's rows #25–#27, proposed in [`23-AUTO-TAGGING-SCOPE.md`](23-AUTO-TAGGING-SCOPE.md) §12, are pending |
| **v3** | Assist | Propose-only VLM · dual-read and review · formula and chart enrichment as `Recognized` or `Proposed` | Assist on and assist off produce byte-identical grounded artifacts |
| **v4** | OCR lane | Deterministic ONNX OCR in-process · an HTTP OCR contract · its own profile · per-page routing · never overwrites `Extracted` · confidence never filtered on | An OCR fingerprint is provably incomparable with a born-digital parse |

### The two gates worth memorising

**v1 tables: do not chase 0.489, and do not chase the hybrid ~0.9 either.** 70‰ is this engine on
twelve tagged PDFs this repository owns. Quote the band with it — 0‰..590‰, median 0‰, ten of twelve
at exactly zero, and removing one document drops the macro to 23‰. An average over mostly zeros is
not a summary of a detector.

0.489 is a published score on somebody else's corpus, and in a different unit — TEDS, not cell-slot F1 ([`table-gate-v1.md`](table-gate-v1.md) §2). The chase is parked
until this repository has a labelled set it owns *and chooses to resume*; v2-S19 built the set, so
the first half is met and the second half is the owner's standing condition (decision #18, decided). The hybrid figure
was never the target and still is not — it is a non-deterministic mode, and a determinism contract
cannot sit under one. **Fabrication 0 still binds, and parking is not passing.**

**v0: byte-identical agreement with the verifier.** Not "close", not "equivalent modulo formatting".
Byte-identical on structure, source binding, representation hash and counts, across all 15 fixtures,
as a CI job rather than a claim.

---

## Not scheduled

Not "later" — not on the roadmap at all. Adding one requires a decision-log entry reversing a settled
decision in [`00-NORTH-STAR.md`](00-NORTH-STAR.md) §2.

| Item | Why not |
| --- | --- |
| PDF/UA export, accessibility studio | A different product for a different buyer. Tag *consumption* improves grounding; tag *generation* does not. **This row read "which is why v2.2 writes tags and stops there" until decision #21**, which refused the writing too. **Decision #23 (2026-09-07) reversed #21**, so the original bound is back: v2.2 may write tags, and PDF/UA export and an accessibility studio stay out of scope above it. What is refused here has never been the writing — it is the *product* built on top of it |
| **Auto-tagging — writing Tagged PDF (v2.2's second half)** | **No longer refused — reopened by [decision #23](00-NORTH-STAR.md) on 2026-09-07, and built ([`23-AUTO-TAGGING-SCOPE.md`](23-AUTO-TAGGING-SCOPE.md)), pending the owner's rows #25–#27 proposed in its §12.** [Decision #21](00-NORTH-STAR.md) had refused it on the format: a `/P` written from a `Computed` cut reads back as `Extracted` and `may_be_overwritten_by` makes it **uncorrectable**. #23 answers the reopening condition with an attribute object under `/A` naming a private owner — in the file rather than in a doc comment, and a shape `structure.rs` parses today. The guarantee is **engine-local**: another reader still sees a plain `/P`, and the format still has no standard field |
| **Declared document splits (D1)** | **Refused on measurement, not principle.** Of five candidate signals three declare navigation or numbering rather than a boundary, and the two that would be honest occur **0** times across all 45 PDF fixtures. A detector with no positive case is an assertion. [`17-D1-SCOPE.md`](17-D1-SCOPE.md) §8 has both reopening conditions |
| **Role-path interning** | **Refused on evidence, not on honesty** — [decision 20](00-NORTH-STAR.md)'s carve-out admits it. It saves 5.98% of a `nist-sp-800-218` artifact and **0.33% of the same artifact compressed**: DEFLATE already back-references the repeated arrays, from outside the contract at no schema cost. [`18-INTERNING-SCOPE.md`](18-INTERNING-SCOPE.md) §5 says where the bytes actually are — 72.25% of the artifact is JSON syntax and key names |
| **Word boxes — sub-run spans with boxes of their own** | **Refused on measurement, not principle** — [`22-WORD-BOXES-SCOPE.md`](22-WORD-BOXES-SCOPE.md). The pinned verifier resolves a quote to the element before any span, so a word box changes a result only for a claim naming its `span_id` or a page-only `value` equal to the word, and every claim this repository verifies cites by element. A five-word quote cited by element loses its precision between the element and its runs on all seven documents measured, not between a run and a word. §7 there says what reopens it, with the owner choosing to resume, and its third condition — §9's reader defects fixed first — is addressed item by item for 0.58.0: items 1 to 4 fixed, item 6 refused rather than mapped, item 5 a documentation defect, with what each left open recorded in that document's amendments. What it recommended instead, turning `char_offsets` on, is the owner's call under §8 there; the owner accepted it on 2026-09-16, and it ships in 0.58.0 |
| **A role, heading or paragraph read off a region** | A region is *where*, never *what*. P14 — and the whole reason layout and structure are separate axes in decision #19 |
| Chart descriptions **as evidence** | A description is `Proposed`. It can exist and can never be cited |
| Any mode that rewrites the evidence | The artifact is the record |
| A public confidence float, at any version | See [`01-CONTRACT.md`](01-CONTRACT.md) §9 |
| A JVM runtime dependency | OpenDataLoader's cost of entry, and the reason its capabilities cannot be borrowed wholesale |
| Converting office files to PDF | **It invents pagination.** Pagination does not exist in a DOCX and must not be synthesised |
| Wrapping another parser as the grounded PDF core | Reference only. See [`06-STEAL-REFUSE.md`](06-STEAL-REFUSE.md) |
| A second verification implementation | See [`07-VERIFY-BOUNDARY.md`](07-VERIFY-BOUNDARY.md). Linking the same verifier is not a second integration; reimplementing its semantics is |
| Any AGPL dependency | Decision #14 |
| Retrieval platforms in the trust core | Rejected on structure, not licence: their table model cannot express row and column spans, so the cross-check cannot run |
| Rankings, "#1", "fastest", or any bake-off table | Every headline number in this landscape is publisher-owned, and one is provably 34 points off depending on invocation flags |

---

## Where the work is specified

| Question | Document |
| --- | --- |
| What is v0, exactly? | [`03-V0-SCOPE.md`](history/03-V0-SCOPE.md) |
| How does v0 get built, and in what order? | [`05-MILESTONES.md`](history/05-MILESTONES.md) |
| What shape must every artifact have? | [`01-CONTRACT.md`](01-CONTRACT.md) |
| Can I borrow feature X from parser Y? | [`06-STEAL-REFUSE.md`](06-STEAL-REFUSE.md) |
| Where does verification live? | [`07-VERIFY-BOUNDARY.md`](07-VERIFY-BOUNDARY.md) |
| What is v1, and did its gate clear? | [`08-V1-SCOPE.md`](history/08-V1-SCOPE.md) / [`09-V1-MILESTONES.md`](history/09-V1-MILESTONES.md) — measured, and **closed on a band by decision #18** |
| What is v1.1? | [`10-V11-SCOPE.md`](history/10-V11-SCOPE.md) / [`11-V11-MILESTONES.md`](history/11-V11-MILESTONES.md) — complete |
| What is v1.2? | [`12-V12-SCOPE.md`](history/12-V12-SCOPE.md) / [`13-V12-MILESTONES.md`](history/13-V12-MILESTONES.md) — complete |
| What is v2? | [`14-V2-SCOPE.md`](history/14-V2-SCOPE.md) / [`15-V2-MILESTONES.md`](history/15-V2-MILESTONES.md) — S0 through S24 done, gate met |
| What is v2.2's first half? | [`16-D4-SCOPE.md`](16-D4-SCOPE.md) — geometric block structure, S0–S4 done |
| What is v2.2's second half? | [`23-AUTO-TAGGING-SCOPE.md`](23-AUTO-TAGGING-SCOPE.md) / [`24-AUTO-TAGGING-MILESTONES.md`](24-AUTO-TAGGING-MILESTONES.md) — auto-tagging: one `/Div` per block of the cut, marked computed under `/A` and read back as `Computed`; the reader (S1) done and the writer (S2) landing in the same version, the round trip (S3) and its measurements (S4) behind them; §7.2's paragraph measurement is already in [`measurements/auto-tagging/`](measurements/auto-tagging/); the owner's rows #25–#27 are proposed in §12 of the scope |
| Why is there no D1? | [`17-D1-SCOPE.md`](17-D1-SCOPE.md) — measured and refused |
| Why is the artifact not smaller? | [`18-INTERNING-SCOPE.md`](18-INTERNING-SCOPE.md) — measured and refused, and where the bytes are |
| How does this engine score on a corpus it does not own? | [`measurements/opendataloader-bench/`](measurements/opendataloader-bench/) — an instrument, never a ranking (O26). NID 0.8471, TEDS 0.1038, MHS 0.0000 at 0.44.0, each with the caveat that makes it readable |
| Can the cut find blocks, not just columns? | [`19-BLOCK-SUBDIVISION-SCOPE.md`](19-BLOCK-SUBDIVISION-SCOPE.md) — measured, three probes run; **not scoped. Decision #21 removed the consumer it was for and #23 gave it back**, so the probes have somewhere to land again |
| Why are there no word boxes, and when did `char_offsets` turn on? | [`22-WORD-BOXES-SCOPE.md`](22-WORD-BOXES-SCOPE.md) — measured and refused for word boxes; `char_offsets` was recommended on the same evidence and **turned on for 0.58.0**, and six reader defects were found on the way |
| What is still open, and what does each item wait on? | [`OPEN-WORK.md`](OPEN-WORK.md) — the ledger: v2.2's second clause, the local plan's v2.3 items (not a roadmap version), owner decisions, recorded defects |

Every version gets a **scope** document and a **milestones** document before it gets code. Nothing
past v2 had one until decision #19, which put v2.2's first half in sight by meeting v2. **The block
cut is the one exception on record**: it shipped at 0.55.0 with `19-BLOCK-SUBDIVISION-SCOPE.md` a
measurement rather than a scope, and the rows above say so rather than backdating one. **v2.2's
second half followed the rule**: its scope, [`23-AUTO-TAGGING-SCOPE.md`](23-AUTO-TAGGING-SCOPE.md),
and its slices, [`24-AUTO-TAGGING-MILESTONES.md`](24-AUTO-TAGGING-MILESTONES.md), were both written
on 2026-09-16, and the code came after both — so the milestones document that waited until the
second half was scoped exists now. **v3 and v4 still should not get one** — a scope document for
work in progress is a plan, and one for work not yet started is a guess.
