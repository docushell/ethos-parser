# The word-box measurement, as it was run

The instruments behind [`../../22-WORD-BOXES-SCOPE.md`](../../22-WORD-BOXES-SCOPE.md), committed so
its numbers can be re-derived rather than believed.

**These are measurement scripts, not product code.** Nothing imports them, they are not on the gate,
and they read artifacts the engine emits rather than reaching into it.

**Run 2026-09-15** with `ethos-parser` 0.57.0 (`target/release/ethos-parser`, built from `e1032a3`)
and the pinned Ethos v0.6.0 (`8adda91`). The corpus figures cover seven `fixtures/gate` documents;
the eighth, `nist-sp-800-53Ar5`, has a 950 MiB representation and is used only where the document
names it — `census.py`, `margin.py`, `ligatures.py`, `spans.py`.

## Running them

```bash
cargo build --release
M=docs/measurements/word-boxes
SEVEN="irs-f1040sd-2025 irs-fw9 nist-sp-800-218 nist-sp-800-207 nist-sp-800-171r3 nist-sp-800-37r2 nist-sp-800-161r1"
mkdir -p /tmp/wb
for f in $SEVEN nist-sp-800-53Ar5; do
  ./target/release/ethos-parser extract fixtures/gate/$f.pdf > /tmp/wb/$f.json
  ./target/release/ethos-parser ground /tmp/wb/$f.json > /tmp/wb/$f.ground.json
done

for f in $SEVEN; do python3 $M/census.py /tmp/wb/$f.json; done
for f in $SEVEN; do
  python3 $M/runs.py  /tmp/wb/$f.json /tmp/wb/$f.ground.json
  python3 $M/words.py /tmp/wb/$f.json /tmp/wb/$f.ground.json
  python3 $M/areas.py /tmp/wb/$f.json /tmp/wb/$f.ground.json
done
python3 $M/joins.py /tmp/wb/irs-f1040sd-2025.json
python3 $M/spans.py /tmp/wb/nist-sp-800-161r1.ground.json

python3 $M/census.py    /tmp/wb/nist-sp-800-53Ar5.json
python3 $M/margin.py    /tmp/wb/nist-sp-800-53Ar5.json 2915
python3 $M/ligatures.py /tmp/wb/nist-sp-800-53Ar5.json
```

`census.py` accepts several representations at once and prints a total, but each of these is loaded
whole into memory, so one at a time is kinder to the machine.

The verifier table and the `char_offsets` evidence:

```bash
E=../ethos-oracle/target/release/ethos
mkdir -p /tmp/wbv
for f in irs-fw9 irs-f1040sd-2025 nist-sp-800-218 nist-sp-800-207; do
  python3 $M/variants.py /tmp/wb/$f.ground.json /tmp/wbv
done
ETHOS_BIN=$E python3 $M/runv.py /tmp/wb/irs-fw9.ground.json /tmp/wbv/irs-fw9.{B_run_offsets,C_word_offsets,D_word_no_offsets,E_box_elsewhere}.ground.json

for a in /tmp/wbv/*.B_run_offsets.ground.json /tmp/wbv/*.NEG_*.ground.json; do
  src=fixtures/gate/$(basename "$a" | cut -d. -f1).pdf
  echo "== $a"
  $E grounding check "$a" --source-artifact "$src"
  ./target/release/ethos-parser grounding-check "$a" --source-artifact "$src"
done
```

And the probes:

```bash
mkdir -p /tmp/wbp && cd /tmp/wbp && python3 "$OLDPWD/$M/probes.py" \
  && for f in p*.pdf; do "$OLDPWD/target/release/ethos-parser" extract $f > ${f%.pdf}.json; done \
  && python3 "$OLDPWD/$M/probe_summary.py" p*.json; cd "$OLDPWD"
```

| Script | What it counts |
| --- | --- |
| [`census.py`](census.py) | Runs, one-character runs, runs with interior whitespace, non-whitespace characters in multi-word runs, visible runs whose advance is 0 and their typed absence, pages declaring `/Rotate`, and code-to-character mismatches |
| [`runs.py`](runs.py) | Each run in a grounded block, classified against the whitespace-delimited words of that block: a fragment of a word, one whole word, several whole words, several cut at an end |
| [`words.py`](words.py) | Each word, classified against the runs that draw it — inside one run, exactly one run, several runs — under two definitions of "needs a cut inside a run": a covering run holds **non-whitespace** outside the word, or holds **any** character outside it, a space included |
| [`areas.py`](areas.py) | Median area of the element box over the union of the run boxes covering a word, and a five-word window. The trimmed-union figures are **estimates**: they apportion a boundary run's width by character count |
| [`joins.py`](joins.py) | Same-baseline joins between consecutive measured runs carrying no whitespace, by gap in box heights, and how many large gaps lie between two runs of dots |
| [`spans.py`](spans.py) | Run spans in a grounding artifact, and the span counts one per word of every run, or of multi-word runs, would reach |
| [`margin.py`](margin.py) | Runs with a positive advance at one origin x, and the state of their boxes. A count by position, used for a note drawn along the page edge under a rotated CTM |
| [`ligatures.py`](ligatures.py) | Runs whose codes decode to more characters than there are codes, beyond synthesized spaces, and the state of their boxes |
| [`variants.py`](variants.py) · [`runv.py`](runv.py) | Grounding variants of one artifact — offsets on today's spans, word spans with and without offsets, a word box moved elsewhere, two negative controls — and nine claims verified against the original and each variant. **Word boxes in the variants are proportional placeholders, not measurements**; they test how the verifier resolves a claim, not where a word sits |
| [`probes.py`](probes.py) · [`probe_summary.py`](probe_summary.py) | Synthetic one-page PDFs, one text-state feature each — `Tc`, `Tw`, `Tz`, `Ts`, rotated and mirrored text matrices, a rotated CTM, `/Rotate`, `TJ` spacing, a width-less code, a ligature, Type 3, Identity-V, and a two-byte `<0020>` under word spacing — and a printer for what `extract` makes of each |

**What is not here.** The document says an independent reader reports all 31,699 visible
zero-advance runs on the seven documents as non-horizontal text, and reads `nist-sp-800-53Ar5`'s
margin note as vertical. That reader was PyMuPDF, run outside the tree: it is AGPL, and decision #14
keeps AGPL out of this repository, instruments included. The probes show the mechanism without it.
