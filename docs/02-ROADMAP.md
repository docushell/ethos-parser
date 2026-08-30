# 02 — Roadmap

One page, deliberately. Nine versions, one line each. No new version numbers get invented — every
change from research folds into a row that already exists.

Five versions have full scope and milestone documents: v0, v1, v1.1, v1.2 and v2. **v2.2, v3 and v4
stay one line each**, which is what "one page" protects.

---

## Versions

| Ver | Theme | Contents | Gate |
| --- | --- | --- | --- |
| **v0** | Honest PDF core | Classify · position-aware text runs · measured ink box or typed absence · synthesized flags · single-column order · format detection · error taxonomy · canonical JSON · capability declarations · grounding emit and check · CLI and library · fuzz and mutation tests | The validator agrees byte-identically with `ethos grounding check` on all 15 fixtures |
| **v0.1** | Verify and robustness | Shell out to the Ethos CLI as a declared capability · encoding-issue detection · the bounded xref repair | An ungrounded claim exits 1 with a report, and nothing is silently skipped. **Met** |
| **v1** | **The DocuShell replacement gate** | Tables, ruled and unruled, with a locator cross-check · vector paths driving ruled detection · full element vocabulary · multi-column reading order · tagged-PDF structure trees · forms and annotations as distinguishable nodes · images · security findings · annotated overlay | Fabrication **0** and an honest table number on the set this repo owns. Measured at 70‰ macro over twelve documents — **a miss**. **Not complete**; decision #18 is written and undecided |
| **v1.1** | Safe Markdown | Markdown and HTML, each with an anchor map · hyphenation and similar cosmetics as export-only | A Markdown-quoted citation verifies end to end |
| **v1.2** | Adoption | MCP server · Python and Node SDKs · LangChain tools | Locators survive every adapter round trip |
| **v2** | Office formats | DOCX, XLSX, PPTX, ODT, ODS, ODP, RTF, EPUB · CSV argued and refused · one shared record and one serializer · embedded assets counted | A DOCX quote and an XLSX cell both **bind** to an address the file states, with **no synthesised pages**. **Met** |
| **v2.2** | Accessibility | Auto-tagging, writing Tagged PDF | A tag this engine writes is one it can read back and ground against |
| **v3** | Assist | Propose-only VLM · dual-read and review · formula and chart enrichment as `Recognized` or `Proposed` | Assist on and assist off produce byte-identical grounded artifacts |
| **v4** | OCR lane | Deterministic ONNX OCR in-process · an HTTP OCR contract · its own profile · per-page routing · never overwrites `Extracted` · confidence never filtered on | An OCR fingerprint is provably incomparable with a born-digital parse |

### The two gates worth memorising

**v1 tables: do not chase 0.489, and do not chase the hybrid ~0.9 either.** 70‰ is this engine on
twelve tagged PDFs this repository owns. Quote the band with it — 0‰..590‰, median 0‰, ten of twelve
at exactly zero, and removing one document drops the macro to 23‰. An average over mostly zeros is
not a summary of a detector.

0.489 is a published score on somebody else's corpus. Same unit, different exam. The chase is parked
until this repository has a labelled set it owns *and chooses to resume*; v2-S19 built the set, so
the first half is met and the second half is the owner's (decision #18, undecided). The hybrid figure
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
| PDF/UA export, accessibility studio | A different product for a different buyer. Tag *consumption* improves grounding; tag *generation* does not, which is why v2.2 writes tags and stops there |
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
| What is v0, exactly? | [`03-V0-SCOPE.md`](03-V0-SCOPE.md) |
| How does v0 get built, and in what order? | [`05-MILESTONES.md`](05-MILESTONES.md) |
| What shape must every artifact have? | [`01-CONTRACT.md`](01-CONTRACT.md) |
| Can I borrow feature X from parser Y? | [`06-STEAL-REFUSE.md`](06-STEAL-REFUSE.md) |
| Where does verification live? | [`07-VERIFY-BOUNDARY.md`](07-VERIFY-BOUNDARY.md) |
| What is v1, and did its gate clear? | [`08-V1-SCOPE.md`](08-V1-SCOPE.md) / [`09-V1-MILESTONES.md`](09-V1-MILESTONES.md) — measured and **missed** |
| What is v1.1? | [`10-V11-SCOPE.md`](10-V11-SCOPE.md) / [`11-V11-MILESTONES.md`](11-V11-MILESTONES.md) — complete |
| What is v1.2? | [`12-V12-SCOPE.md`](12-V12-SCOPE.md) / [`13-V12-MILESTONES.md`](13-V12-MILESTONES.md) — complete |
| What is v2? | [`14-V2-SCOPE.md`](14-V2-SCOPE.md) / [`15-V2-MILESTONES.md`](15-V2-MILESTONES.md) — S0 through S24 done, gate met |

Every version gets a **scope** document and a **milestones** document before it gets code. Nothing
past v2 has one, and v2.2, v3 and v4 should not get one until v2's successor is in sight.
