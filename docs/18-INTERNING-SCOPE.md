# 18 — Role-path interning: measured and refused

**Scope authority for interning.** There are no milestones, because there is no work. This document
records what was proposed, what was measured, and why the measurement ends it.

**Refused on evidence, not on honesty.** That distinction is the first thing to read, and §3 is
where it lives: [decision 20](00-NORTH-STAR.md) does **not** forbid this, and nothing here borrows
row 20's authority for a judgment row 20 explicitly declined to make.

---

## 1. The one sentence

**DEFLATE already does what an intern table does — from outside the contract, at no schema cost.**

## 2. What was proposed

`role_path` repeats enormously. On `nist-sp-800-218` — 36 pages, a 29,979,752-byte artifact —
there are **21 distinct role paths across 59,682 uses**, the commonest being
`["Sect","Table","TR","TD","P"]` 34,617 times. On `irs-fw9`, 13 distinct across 1,097 uses.

The proposal: a document-level table in `RepresentationPayload`, and each node carrying an index
into it. `ci/bench.py` had established that run time is linear in emitted bytes, so fewer bytes is
less time, and this was the largest saving available that removes no claim from the artifact.

## 3. It is admissible under decision 20, and that must not be misread

Decision 20 refuses omit-and-restore-a-default. It carves out, in its own words, *"encodings that
keep the value **inside** the artifact and resolvable from it alone — the value is still stated, by
reference rather than by repetition."*

Interning satisfies that literally. The role path stays in the artifact byte for byte, inside the
subtree `representation_c14n_sha256` covers; nothing is derived, defaulted, or restored at read
time; decision 20's reopening condition — a contract-level derivation rule — does not bite, because
interning derives nothing.

**The "a node stops being readable alone" objection is a real cost and is not decisive**, because
the artifact already declines that property in three places: `Node` carries no geometry at all (it
is an index-aligned sidecar), `Node.parent` is `"p3"` until you consult `pages[]`, and the record
says the principle outright at a neighbouring variant — *"a node cannot see the document."*

So if interning were free it would be defensible. It is not free, and §4 is why it is also not
worth anything.

## 4. The measurement

Raw bytes saved, and then the same saving after the transport anyone actually uses:

| | artifact | raw saving | gzip −6 | **saving on the compressed artifact** |
| --- | --- | --- | --- | --- |
| `irs-fw9` | 827,349 B | 15.14% | 90,698 B | **2.64%** (2,396 B) |
| `nist-sp-800-218` | 29,979,752 B | 5.98% | 1,870,474 B | **0.33%** (6,188 B) |

**The artifact gzips to 6.2% of itself.** DEFLATE's LZ77 back-references the repeated arrays — which
is precisely what an intern table does, done outside the contract, costing no schema version, no new
invariant and no consumer change. Interning then buys a fraction of a percent of what anyone stores
or moves.

The one thing it buys that compression does not is **throughput**, because run time is linear in
emitted bytes. That is also small: 5.98% of a 30 MB artifact is roughly 23 ms on a 450 ms extract.

**And "about 5%" would be the wrong number to publish in any case.** Across the eight gate documents
the raw saving is bimodal — 15.14% on a 6-page form, 5.98% on a 36-page report — because the saving
tracks how few distinct paths a document uses, not how large it is. Quoting a mean over that shape
is the reporting failure [decision 18](00-NORTH-STAR.md) exists to prevent.

## 5. What the measurement found instead, and it outlasts this row

On `nist-sp-800-218`:

| | share of artifact |
| --- | --- |
| JSON syntax and key names | **72.25%** |
| all values, of every kind | 27.75% |
| **the document's own text** | **0.77%** |

A 30 MB artifact carries **230 KB** of text — the format is roughly 130 times the content it
describes. Within that, `structural_locator` is 17.73% and is not even the largest per-node field;
`attributes` is 26.50%. And `structural_locator`'s own bulk is not the paths: the **envelope**
— `"structural_locator":{"pdf_tagged":{"mcid":N,"role_path":[…]}}`, about sixty characters of
scaffolding per node — is **11.84%**, against 5.15% for the path values interning would compress.

**No single-field encoding change moves a number whose dominant term is the shape of the format.**
That is the finding, and it is worth more than the row it came from.

## 6. What interning is not

- **Not refused on honesty.** §3. A future proposal may cite this document for the measurement and
  must not cite it for a principle it does not contain.
- **Not a saving on three of the four artifact families.** `ground` carries no structural locator
  at all, and Markdown and HTML carry no role path. Only `extract` shrinks.
- **Not visible to the instrument that would have judged it.** `ci/bench.py` benches `extract`
  alone, so the win would land inside the harness while any projection cost stayed outside it.
- **Not a reason to stop compressing.** A caller writes `ethos-parser extract x.pdf | gzip` today
  and gets 16×, with no engine change and no contract risk. That is the answer to "the artifacts are
  enormous", and it was always available.

## 7. One asymmetry, recorded for the next proposal

Every intra-artifact reference the record already uses points at a **record with its own identity**
— `parent` → a `PageRecord`, `node_ids` → a `Node`, a cell's ids → a `TableRecord` — and
`NodeGeometry` carries **two** witnesses, index alignment *and* the node id, both checked, on the
standing rule that *redundancy that is tested is a cross-check; redundancy that is not is two places
for the truth to live.*

An index into a bare value table has no second witness. A one-byte flip turning `"role":7` into
`"role":1` yields a well-formed artifact asserting a **different valid** structural address. The
same flip inline yields `"Tabme"`, which is visibly not a PDF structure type.

Decision 20's carve-out does not say whether a referent must be a record. This row did not need that
question answered. **The next interning proposal will.**

## 8. What would reopen it

Both halves, not either:

1. **A transport where compression is not available**, or a consumer that demonstrably must hold raw
   canonical bytes at scale — because against gzip the saving is a fraction of a percent, and that
   is the whole argument.
2. **The owner choosing to resume**, on a number quoted as a band rather than a mean.

**Neither is met.** And if artifact size is being attacked again, §5 says where the bytes actually
are, and it is not here.

## 9. Standing rules this decision keeps

1. **A measurement decides, and the band is published with it** — never a mean over a bimodal shape.
2. **A refusal records what it does *not* refuse**, so the next reader inherits the evidence rather
   than the conclusion.
3. **No new invariant is created for a saving that the transport already provides.**
