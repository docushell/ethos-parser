# ethos-parser against liteparse, on opendataloader-bench

**Measured 2026-09-20** on an Apple M4 Pro: `ethos-parser 0.59.0` (the release build of the 0.59.0
commit) against `lit 2.14.6`, over the 200 documents of `opendataloader-bench`, scored by that
harness's own three evaluators. Instrument: [`headtohead.py`](headtohead.py); per-document readings:
[`results.json`](results.json).

**It is not published, and it is not a ranking.** `docs/06-STEAL-REFUSE.md` O26 refuses "#1, fastest,
or any bake-off claim"; running one internally needs no decision, and publishing it is North Star
D3, which the owner has not taken. This page exists so that the gap is a measured number in this
repository rather than an impression.

**liteparse is installed, never vendored.** `cargo install liteparse --no-default-features`, which
is Apache-2.0, as are its `liteparse-pdfium` crates; nothing of it enters this tree, and the
instrument shells out to the binary. Decision #14 is unaffected.

---

## 1. What was run

```bash
cargo install liteparse --no-default-features          # 2.14.6; no tesseract feature, so no OCR
ETHOS_BENCH=~/ethos-external-benchmarks/opendataloader-bench \
ETHOS_PARSER_BIN=target/release/ethos-parser \
  $ETHOS_BENCH/.venv/bin/python docs/measurements/liteparse-head-to-head/headtohead.py
```

Each engine runs as its own CLI, one document at a time. ethos-parser goes through `extract` then
`markdown` — the path the harness's own adapter uses, two processes with the representation on
disk. liteparse goes through `lit parse --format markdown --no-ocr -q`. Wall seconds are measured
around each process and peak RSS comes from `wait4`, so the memory figure is the process's own; for
ethos-parser the seconds are the two processes' sum and the peak is the larger of the two, which is
what a caller pays for one Markdown file.

**liteparse under three flag sets**, because its defaults decide what reaches the Markdown and
ethos-parser has no equivalent knob — it emits the projection verbatim:

| set | flags | what it changes |
| --- | --- | --- |
| `default` | none | strips running headers and footers, emits an `![](img_pN_K.png)` per raster image, renders link annotations as `[text](url)` |
| `parity` | `--no-links --image-mode off` | the two its own `--help` names as being for benchmark parity, "where ground truth uses no link syntax" |
| `furniture` | `parity` + `--keep-headers-footers` | keeps the page furniture, which is the content selection closest to what this engine does |

**DP-Bench was not run**: no copy of it exists on this machine.

## 2. Quality — bands, with the worst document named (decision #18)

| metric | n | ethos-parser 0.59.0 | liteparse 2.14.6 (`furniture`) |
| --- | ---: | --- | --- |
| **NID** reading order | 200 | **0.8694** · median 0.9238 · band 0.0068..0.9973 · worst `01030000000141` | **0.9190** · median 0.9569 · band 0.0068..1.0000 · worst `01030000000141` |
| **TEDS** table structure | 42 | **0.1704** · median 0.0000 · band 0.0000..0.9802 · worst `01030000000064` · **28 of 42 zero** | **0.8179** · median 0.8852 · band 0.0000..1.0000 · worst `01030000000110` · 2 of 42 zero |
| **MHS** headings | 107 | **0.3321** · median 0.1490 · band 0.0000..0.9986 · worst `01030000000001` · 50 of 107 zero | **0.8234** · median 0.9419 · band 0.0000..1.0000 · worst `01030000000107` · 4 of 107 zero |

Per document, not pooled: **liteparse scores higher on 165 of 200 for NID, 40 of 42 for TEDS and 93
of 107 for MHS.** ethos-parser scores higher on 34 NID documents — its largest lead is
`01030000000018`, +0.4443 — on no TEDS document, and on 9 MHS documents, the largest
`01030000000067` at +0.5019.

**The flag sets barely move it**: NID 0.9123 (`default`) → 0.9172 (`parity`) → 0.9190 (`furniture`),
TEDS 0.8179 throughout, MHS 0.8210 → 0.8234. Its defaults cost it about seven thousandths of NID
against this ground truth, and nothing else.

## 3. Speed and memory

| | ethos-parser 0.59.0 | liteparse 2.14.6 (`furniture`) |
| --- | --- | --- |
| seconds/document | **0.035** mean · median 0.032 · band 0.022..0.151 · worst `01030000000141` | **0.026** mean · median 0.023 · band 0.015..0.104 · worst `01030000000141` |
| peak RSS | **9.7 MB** mean · median 8.6 · band **6.3..170.8** · worst `01030000000141` | **25.1 MB** mean · median 25.0 · band 22.1..43.8 · worst `01030000000141` |

ethos-parser is slower on 164 of the 200 documents and lighter on the median one by 2.9×, and its
memory band is the wider by far: on `01030000000141` it peaks at 170.8 MB against liteparse's 43.8
MB. That document is also the worst NID document for both engines, and the slowest for both in this
flag set; `default`'s slowest is `01030000000036` at 0.224 s.

## 4. Two controls, because a head-to-head is mostly a test of the harness

- **The ethos column reproduces this repository's own scorer exactly.** NID 0.8694, TEDS 0.1704 and
  MHS 0.3321 are the figures [`../opendataloader-bench/README.md`](../opendataloader-bench/README.md)'s
  2026-09-18 appendix records for `type-size-v2` — a different instrument, run a different way, to
  the same four decimal places.
- **liteparse's own stored run is in the harness**, and the same evaluators score it NID 0.8660,
  TEDS 0.0000, MHS 0.0000 (`prediction/liteparse/`, version 1.2.1, 2026-04-06). So 2.14.6 is a large
  change from what that corpus recorded, and the numbers above are not an artefact of driving the
  CLI oddly: they also sit beside the
  figures liteparse published for this corpus when this repository last read its README on
  2026-09-08 — NID 0.908, TEDS 0.693, MHS 0.816, for whatever version was current then — rather
  than above or below them by a margin that would suggest the CLI was driven wrongly.

## 5. What this does not measure, which is most of what this engine is for

- **Everything the contract carries is flattened to a string before any metric runs**: the profile
  and its hash, the declared coordinate system, integer centipoint quanta, typed absence, the
  derivation class, the anchor map that makes a quote bindable, the grounding artifact, and every
  refusal. The harness's own adapter docstring says the same thing in its first paragraph. **This
  scores two Markdown emitters.**
- **No metric here penalises a claim the document does not support.** All three reward output that
  matches a ground truth; none of them can see a fabricated heading, an invented table cell or a
  span bound to the wrong box. The measurements that do are this repository's own — the heading
  rule's false-positive bound ([`../headings/README.md`](../headings/README.md)) and the table gate.
- **One host, one run per document**, no repetition: the speed and memory bands are single samples,
  and the engines were run in the same session but not interleaved.
- **OCR is off**, and this build of liteparse has no tesseract feature at all. A corpus of scans
  would measure something else entirely.
- **The corpus and the ground truth are someone else's**, 200 born-digital documents; TEDS is scored
  on the 42 whose ground truth holds a table and MHS on the 107 that hold a heading.
- liteparse's other knobs — `--dpi`, `--target-pages`, `--extract-blocks`, `--extract-structure-tree`,
  `--preserve-small-text` — were not swept.

## 6. What it says about the tracks already open here

Read as a gap list rather than a verdict, each item already has a record in this repository:

1. **Tables are the largest gap** — 0.1704 against 0.8179, with 28 of 42 documents at zero. The
   cause is measured in [`../opendataloader-bench/tables-why-zero.md`](../opendataloader-bench/tables-why-zero.md):
   on 11 of the 28 the unruled rule projects every run on the page onto two axes and the candidate
   is refused by the 4,096-cell ceiling. `OPEN-WORK.md` §4 holds the decision that would narrow it.
2. **Headings are the second** — 0.3321 against 0.8234. Decision #29's rule took this off 0.0000
   on 2026-09-18; what remains is that it emits one level and infers only where a document
   declares no structure, while liteparse labels levels.
3. **Reading order is the closest** — 0.8694 against 0.9190. The identity fallback is the known
   cause, and `OPEN-WORK.md` §4's undecided row measures the repair at 0.9162 on this same corpus,
   which would close most of the remaining distance.

---

## 7. Amended 2026-09-20, after optimising against these numbers

The owner asked for all four to beat `lit`, and chose the bound the work had to stay inside: only
changes that cannot invent structure. **One of the four is now ahead; the other three are held by
that bound, and this section says exactly what each would cost.** Re-measured by the same
instrument, same corpus, same machine, with `ethos-parser` at `edcf85f`:

| metric | ethos-parser before | ethos-parser now | liteparse 2.14.6 |
| --- | --- | --- | --- |
| seconds/document | 0.035 (two processes) | **0.020** · median 0.018 · band 0.013..0.136 | 0.024 |
| peak RSS | 9.7 MB | **9.4 MB** mean · median 8.4 | 25.1 MB |
| NID | 0.8694 | 0.8714 · median 0.9243 | 0.9190 |
| TEDS | 0.1704 | 0.1704 | 0.8179 |
| MHS | 0.3321 | 0.3353 · median 0.1565 | 0.8234 |

**Speed: ahead, by changing the shape of the call rather than the engine.** `tests/pipeline_cost.rs`
measured the 0.035 s and found 5.4 ms of engine work under ~15 ms of process starts and ~3 ms of
carrying a 250 KB record between two processes. `markdown --source` runs both stages in one process
and emits byte-identical bytes; the engine's own work was never the slow part, and `lit`'s is ~15 ms
against this engine's 5.4.

**Reading order: +0.0020, and the measured ceiling is out of reach inside the bound.** The gain is
the block-join repair (`markdown-blocks-v9`): a page set with tracking drew each glyph as its own
run, and every letter became its own block — `01030000000103` projected 944 blocks of 1.2 characters
and scored 0.4456; it now projects 48 and scores 0.5708. 58 documents moved, none down. What remains
is `reading-order-causes.md`'s ordering cost, worth +0.0465 if driven to zero, and it lives on
single-column pages whose *content stream* is out of order. The fix prescribed there — peel a
full-width band, then look for gutters in what remains — was built and measured: it reaches 4
documents and is **net −0.1042**, because the 178 identity-arm documents are not hiding columns.
Reordering them needs a rule that reorders on position alone, which `reading_order.rs` refuses in
its header and the measurement refuses again. Reverted, and recorded here instead.

**Headings: the zeros are not near the cut, they are at body size.** A diagnostic build printed the
rule's own view on the documents scoring 0: `body_em=1100` with candidate line ems of exactly `{900,
1100}` — the headings those documents draw are the *same size* as their body text, set bold. No size
rule can reach them at any cut, which the sweep confirms: 1.20× → 1.15× moves MHS 0.3353 → 0.3455
and the zeros 49 → 46, and 1.10× makes MHS worse. The only remaining signal is the font, and
`28-HEADINGS-SCOPE.md` §7.3 measured the size-or-font disjunction at up to **15.76%** false
positives against the **5%** bound the owner accepted — three times over. A narrower signal (the
font's own `/FontDescriptor /Flags` ForceBold bit, or a `/BaseFont` name that says Bold) is the one
untried path that could come in under the bound, and it is untried: it needs building and then
measuring on the eleven tagged documents before anything ships.

**Tables: the arithmetic says no, inside the bound.** The 28 zeros break down as 2 documents whose
truth transcribes a picture (OCR, which the profile refuses), 12 where a drawn grid was built and
refused, 11 past the 4,096-cell ceiling and 3 under the gutter floor
(`../opendataloader-bench/tables-why-zero.md`). Fixing only the causes that cannot fabricate — the
ceiling and the floor, 14 documents — and lifting today's 14 non-zero tables to 0.8 reaches about
**0.60**. Passing 0.8179 needs the 12-document relaxation, and `table-gate-v1.md` records that
relaxation (`-v5`) fabricating a table on **five of eight** gate documents. That is the trade the
owner declined, so it was not built.
