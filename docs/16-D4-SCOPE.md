# 16 — D4 scope: geometric block structure

**Scope authority for D4.** Slices are in §9 and every D4 PR belongs to exactly one.

**This is the first half of v2.2**, by decision #19 of 2026-08-30. §7 is the argument the owner
accepted. No version number was invented: v2.2 already existed, and this is the half of it nobody
had scoped.

**There is no milestones document, deliberately.** [`02-ROADMAP.md`](02-ROADMAP.md) pairs one with
each *version*, and v2.2's second half — auto-tagging — is not started. A milestones document
covering half a version would be a plan for the half in progress and a guess for the other. It gets
written when auto-tagging does.

---

## 1. The one sentence

**The cut already knows where the blocks are, and throws it away.**

Everything below is machinery for keeping it.

## 2. Why this version exists

`gutter-columns-v1` cuts a page into column bands, cuts each band into blocks, and offers each
block back to the column cut. Then it returns a **permutation** — the order — and discards the
structure that produced it ([`reading_order.rs`](../crates/ethos-parser-pdf/src/reading_order.rs), `order`).
A consumer receives the runs of a two-column page in the right sequence and cannot tell that the
page had two columns.

The consequence shows up in the projections. Headings and list items reach Markdown **only** from
the tagged structure tree ([`markdown.rs`](../crates/ethos-parser-core/src/markdown.rs): *"A heading is
a heading because the structure tree said so"*), which is correct and stays correct — a role read
off a font size is P14. But it means an **untagged** visually complex document projects as one flat
run of paragraphs, because nothing else on the wire says where anything begins or ends.

The engine measured this page's structure and then refused to say so. That is the gap, and it is
not a detection problem: the measurement already happened.

## 3. The distinction this version exists to hold

**Layout is where text sits. Structure is what text means.** ADE and the parsers around it conflate
the two — a model looks at a page and returns `"type": "text"` chunks whose role and whose
rectangle come out of the same inference. This version keeps them apart, and the separation is the
capability:

| | Answers | Source | Derivation | Absent when |
| --- | --- | --- | --- | --- |
| **Layout** (this version) | *where does this sit* | whitespace this engine measured | `Computed` | the cut found no division |
| **Structure** (v1-S3, unchanged) | *what is this* | the document's `/StructTreeRoot` | `Extracted` | the document is untagged |

A region is **never** a paragraph, a heading, a section or a column. It is the region the cut made.
Anything more is a claim about meaning, and meaning comes from the tree or from nowhere.

## 4. What lands on the wire

One optional field on [`Node`](../crates/ethos-parser-core/src/representation.rs), beside `ordinal`:

```rust
/// Which region of its parent the reading-order cut placed this node in, 1-based, in reading order.
#[serde(skip_serializing_if = "Option::is_none")]
pub region: Option<u32>,
```

| Value | Means |
| --- | --- |
| `Some(n)` | the cut divided this page, and this node is in its `n`th region, counted in reading order |
| absent | **the cut made no division here** |

Four decisions are packed into that, and each is load-bearing.

**Beside `ordinal`, inside the fingerprint.** `NodeGeometry` sits outside the digest on purpose —
boxes are the least reliable number in the record and identity must not be hostage to them. A
region is not a box. It comes from the same cut that produces `ordinal`, which *is* fingerprinted,
and the two can never disagree because one rule emits both. Putting the region outside the digest
would mean the Markdown projection decided its block boundaries from a number the fingerprint does
not cover, which is the seam §4 of the contract warns about, pointed the wrong way.

**Absent rather than `1`.** A single-column page is the overwhelmingly common case, and the module
already says the identity permutation is *"a real answer, not a fallback state"*. `region: 1` on
every node of such a page would be a field that never varies and never informs. Absence is the
established idiom here — `standard_role_path` is *"present only when the map actually changed
something, so its presence is the signal"* — and it is also the whole performance argument (§6).

**A bare `Option`, and not a typed absence — which needs arguing, because this repository refuses
one elsewhere.** [`derivation.rs`](../crates/ethos-parser-core/src/derivation.rs) is explicit that
`GeometryPresence` is *"deliberately **not** `Option<QRect>`"*, because `None` there would collapse
*"we could not measure"*, *"there is nothing to measure"* and *"we were not asked to measure"* into
one — **and the first is a capability limitation that must be declared and counted.**

Absence here collapses four states too: no gutter met the rule, fewer than two runs on the page,
the capability is off, and the format has no cut at all. **The difference is where the
discriminator lives.** A box's absence varies node by node *within one document*, so only a
per-node type can carry it. A region's absence does not: every one of those four is a property of
the profile or the page, and `reading_order_rule` already names which rule ran —
`READING_ORDER_RULE_V0` for a profile with the capability off, and each office format's own id.
Spending a tagged enum on every node to restate one profile field would be per-node cost for a
document-level fact.

This is the `PdfMcid` precedent rather than the `GeometryPresence` one: *"the assurance block's
limitations say which; this variant alone does not, because a node cannot see the document."* Both
patterns are live in this codebase, and the test for which applies is whether the states being
collapsed can differ between two nodes of one artifact. Here they cannot.

**Absent means the cut made no division — it does not mean "single column".** Those coincide on
almost every real page and they are not the same claim, and nothing in the projections may read the
second from the first.

**1-based**, matching `ordinal` and the per-page convention the classifier already uses.

## 5. What D4 is not

- **Not a new node kind.** The standing rule from v1-S1 and v1-S3 is *do not add a kind for a fact
  some existing node already carries* — and its converse: a fact no node carries needs a field, not
  a kind. `Paragraph` and `Heading` were refused as kinds and stay refused.
- **Not a role, a heading level, or a paragraph.** See §3. If this version ever emits a name for
  what a region *is*, it has become P14.
- **Not a change to reading order.** The permutation is byte-identical before and after. This is
  the acceptance criterion of the first slice, not an aspiration.
- **Not a new detector.** No new geometry is computed, no new threshold is introduced, and no
  constant moves. Every number stays where `gutter-columns-v1` put it.
- **Not a nesting tree.** The cut recurses — `arrange_columns` and `arrange_blocks` call each other
  to `MAX_CUT_DEPTH` — so the honest full answer is a *path*, and a flat ordinal is a projection of
  it. §8 records that as the known limitation, with what would justify widening it.
- **Not applicable to office formats.** They have no cut. The field is absent there, permanently,
  and the profile says why.

## 6. Performance, which is the priority this version was scoped under

`ci/bench.py` establishes the shape of the cost: **the engine is linear in what it emits**, at 0.015
s/MB of output, flat across all six large gate documents. `nist-sp-800-53Ar5` is 7.5 MB in and 932
MB out, and that expansion is the run time.

So the performance question for D4 is not *"does it add a pass"* — it does not; the cut already
runs, and the region falls out of the recursion that is already executing. **The performance
question is bytes per node.**

That is what §4's absence rule buys. A single-column page emits nothing, so the common case costs
**zero bytes and zero time**. A cut page pays roughly twelve bytes per node, which on the corpus is
bounded and measurable rather than argued.

**The acceptance criterion is a reading, not a claim:** `ci/bench.py --check` against the baseline
recorded before the first slice, on the same machine, within its 10% tolerance. A slice that cannot
show that is not done.

## 7. Where this belongs on the roadmap

**DECIDED 2026-08-30 — decision #19: the first half of v2.2, and not a new row.** v2.2 is renamed
*layout and accessibility*, and its gate now has two clauses. The argument the owner accepted:

v2.2 is auto-tagging — *writing* Tagged PDF, gated on *"a tag this engine writes is one it can read
back and ground against"*. **You cannot write a tag for an untagged document without first deciding
where its blocks are.** D4 is that decision, made geometrically and emitted as evidence rather than
kept as a private intermediate. It is the missing first half of a row that already exists, which is
what [`02-ROADMAP.md`](02-ROADMAP.md)'s *no new version numbers get invented* asks for.

The dependency also runs the right way: D4 emits `Computed` geometry and claims nothing about
meaning; v2.2 would later attach names to those regions and must justify each name separately.
Shipping D4 first means the geometric half is on the wire and measurable **before** anything starts
naming it — and if v2.2 never happens, D4 is still worth its own bytes.

**What the decision carries with it**, accepted rather than discovered later: the rule id moves to
`gutter-columns-v2` (§10), so `profile_sha256` moves and every golden regenerates. Contract §2
requires exactly that — *anything that can change a byte of output belongs in the profile, or it is
a bug.*

**The second half stays unscoped.** Auto-tagging gets its milestones document when somebody starts
it, not now.

## 8. Known limitation, declared rather than discovered later

**A flat ordinal is a projection of a tree.** The cut alternates axes to `MAX_CUT_DEPTH`, so a
region is really addressed by a path — column 0, block 1, column 1 — and `region: 4` is that path
flattened into reading order. Two nodes with different regions are genuinely in different regions;
two nodes with the same region are genuinely in the same one. What is lost is *why* the boundary
between them exists and how deeply it nests.

This ships flat, deliberately. The flat form answers the question the projections actually ask —
*does a block end here* — at a fixed twelve bytes, and the path form costs a variable-length array
on every node of every cut page, on an artifact whose size is already the run time. **Widening it
needs a consumer that demonstrably cannot work with the flat form**, not a sense that more
information is better.

The limitation is declared on the artifact, not only here.

## 9. Slices

| Slice | Theme | State |
| --- | --- | --- |
| **S0** | This document and the milestones | done |
| **S1** | `arrange_page` returns regions beside the permutation; ordering byte-identical; no wire change | done |
| **S2** | `region` on `Node`, absent where no cut; schema, fixtures and goldens | |
| **S3** | Markdown and HTML project region boundaries; anchor map still tiles | |
| **S4** | The declared limitation, and `ci/bench.py --check` green against the S0 baseline | |

S1 lands the whole mechanism behind no wire change at all, so the claim *reading order did not
move* is provable by `diff` before anything downstream can be blamed for it.

## 10. Identity

- **The rule id moves: `gutter-columns-v1` → `gutter-columns-v2`.** The cut is unchanged and the
  order is unchanged, but an artifact under the id gains a field, and
  [`html.rs`](../crates/ethos-parser-core/src/html.rs) has already settled what to do about that:
  *two builds in this repository's own history producing different bytes under one id* is the state
  a rule id exists to make impossible, and *a version that is cheap to move is exactly the one worth
  moving.* `READING_ORDER_RULE_V0` and `V1` keep their meanings.
- **No second rule id.** A separate `region_rule` was considered and refused: `html_rule` is
  separate from `markdown_rule` because the two can move independently, and these two cannot. One
  cut emits both, and two ids for one rule would claim a precision that does not exist.
- **No new capability flag.** `multi_column_reading_order` already gates the cut, and a region is
  what that cut produces. A second flag would be a flag no consumer can act on independently.
- **`profile_sha256` moves**, on the rule id as well as on `parser_version`.

## 11. Standing rules, carried forward

1. **No role, no heading and no box derived from a font size** — and none derived from a region
   either. A region is where, never what.
2. **No invented coordinate, identifier or boundary.** Every region boundary is a gap the rule
   measured in page space.
3. **A gap is never presented as a success** — a page the cut did not divide says so by absence,
   and the profile says which absence it is.
4. **Byte identity across runs**, and across this change for the ordering itself.
5. **No public confidence field**, on a region or anywhere else.
