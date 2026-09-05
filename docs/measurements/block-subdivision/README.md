# The block-subdivision measurement, as it was run

The instrument behind [`../../19-BLOCK-SUBDIVISION-SCOPE.md`](../../19-BLOCK-SUBDIVISION-SCOPE.md),
committed so the numbers in that document can be re-derived rather than believed.

**These are measurement scripts, not product code.** They are not on the gate, nothing imports them,
and they read artifacts this engine emits rather than reaching into it. `17-D1-SCOPE.md` and
`18-INTERNING-SCOPE.md` were argued from measurements whose instruments live off-tree, and a reader
who wants to check them cannot; this is the correction.

## Running them

```bash
cargo build --release
mkdir -p /tmp/bsm
for f in nist-sp-800-207 nist-sp-800-218 irs-fw9 irs-f1040sd-2025; do
  ./target/release/ethos-parser extract fixtures/gate/$f.pdf > /tmp/bsm/$f.json
done
for f in nist-sp-800-171r3 nist-sp-800-37r2; do
  ./target/release/ethos-parser extract --max-pages 60 fixtures/gate/$f.pdf > /tmp/bsm/$f.json
done

python3 docs/measurements/block-subdivision/mcid_probe.py /tmp/bsm/*.json
python3 docs/measurements/block-subdivision/gap_dist.py   /tmp/bsm/*.json
python3 docs/measurements/block-subdivision/labelled.py   /tmp/bsm/*.json
python3 docs/measurements/block-subdivision/gap_samples.py /tmp/bsm/nist-sp-800-207.json
```

`--max-pages` moves `profile_sha256`, which does not matter here: nothing compares these artifacts
to a golden. It bounds two documents whose full artifacts run to hundreds of megabytes.

## What each answers

| script | question | answer it gave |
| --- | --- | --- |
| `mcid_probe.py` | is a PDF `mcid` a paragraph unit or a line unit? | **line** — median one baseline per mcid on five of six documents, so it cannot label paragraphs |
| `gap_dist.py` | are baseline gaps bimodal within a column band? | yes, once normalised by the band's own leading |
| `labelled.py` | can a threshold separate boundaries the structure tree declares? | yes for headings and lists, **and the label set contains no `P → P` pair at all** |
| `gap_samples.py` | what *is* a gap of each size? | prints the text either side, which is how the `P → Link` error was caught |

## Reading them adversarially

Each carries its own defects in its docstring, including three the author found after believing the
output once. `labelled.py`'s `INLINE_ROLES` is the fix for the second, and it has a known gap
recorded here rather than silently: **`Lbl` is listed as inline and PDF 32000-1 §14.8.4.3 makes it
block-level.** It occurs zero times in this corpus, so it moves no number quoted in `19`, and it
would fire on any document that tags its list markers.

The fourth defect is **not** fixed: `(page, region)` interleaves table columns, so 21 of 232 bands
are not text flows and five `nist-sp-800-218` pages report a modal "leading" of 2 points that is a
column interleave pitch. It makes the numbers pessimistic, and §3 of `19` says so.
