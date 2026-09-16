# The five-knob measurement, as it was run

The instruments behind [`../../25-KNOBS-SCOPE.md`](../../25-KNOBS-SCOPE.md), committed so its
numbers can be re-derived rather than believed. The results tables are in that document; this page
says what each instrument measures, how it was run, and what it did not measure.

**These are measurement scripts, not product code.** Nothing imports them, they are not on the gate,
and they read artifacts the engine emits or inspect PDFs with qpdf. No corpus file is committed.

**Run 2026-09-16** against the tree at `b4b4aa9` (0.58.0 unreleased) with:

- **Engine:** `target/aarch64-apple-darwin/release/ethos-parser`, which prints `ethos-parser 0.58.0`;
  SHA-256 `03b752ed42b469f3cbd628cac0a037ed046a9718d200d518cd9d11c4a40e8513`, byte-identical to the
  binary inside `target/release-artifacts/ethos-parser-0.58.0-aarch64-apple-darwin.tar.gz`, which
  `SHA256SUMS.txt` there marks `verified`. `target/release/ethos-parser` on the same host prints
  0.57.0 and is stale; a first pass taken with it was discarded and every engine reading below was
  re-taken with the 0.58.0 binary. The qpdf census does not run the engine and was not re-taken.
- **qpdf 12.3.2** at `/opt/homebrew/bin/qpdf` (Apache-2.0), used out of band as a second reader.
- **Host:** Apple M4 Pro, 12 cores, 48 GiB, macOS, shared with other work — load average 2.0 to 3.1
  before and after the timing runs. Python 3.9.6.

## The corpora

297 PDFs, listed by [`corpora.py`](corpora.py) from environment-settable roots. The oracle root must
be set when the repository is not checked out beside `ethos-oracle`.

| corpus | root | PDFs | pages (qpdf) |
| --- | --- | ---: | ---: |
| gate | `fixtures/gate` | 8 | 1,466 |
| engine | `fixtures/engine` | 44 | 44 |
| oracle | `../ethos-oracle/fixtures` | 35 | 52 readable, 1 unreadable |
| gate-zero | `../ethos-oracle/benchmarks/gate-zero/corpus` | 10 | 608 |
| odl-bench | `~/ethos-external-benchmarks/opendataloader-bench/pdfs` | 200 | 200 |

Not included: the OmniDocBench pages (images, not PDFs — see the omnidocbench measurement), and the
office fixtures, which are not PDFs and have their own encryption declarations (`lib.rs`,
`epub.rs`).

## Running them

```bash
M=docs/measurements/knobs
export ETHOS_PARSER_BIN=target/aarch64-apple-darwin/release/ethos-parser
export ETHOS_ORACLE=../ethos-oracle QPDF=qpdf
export KNOBS_TMP=/tmp/knobs            # where extract writes each artifact while it is read

python3 $M/corpora.py | cut -f1 | sort | uniq -c          # 44 engine, 8 gate, 10 gate-zero, 200 odl-bench, 35 oracle

python3 $M/qpdf_census.py   run --out $M/results/qpdf-census.tsv
python3 $M/qpdf_census.py   summary $M/results/qpdf-census.tsv
python3 $M/engine_census.py run --out $M/results/engine-census.tsv
python3 $M/engine_census.py summary $M/results/engine-census.tsv

mkdir -p $KNOBS_TMP/gate7 && for f in irs-f1040sd-2025 irs-fw9 nist-sp-800-218 nist-sp-800-207 \
    nist-sp-800-171r3 nist-sp-800-37r2 nist-sp-800-161r1; do
  ln -sf "$PWD/fixtures/gate/$f.pdf" $KNOBS_TMP/gate7/; done
python3 $M/batch.py ~/ethos-external-benchmarks/opendataloader-bench/pdfs --parallel 4 --parallel 8 --repeat 3 --label odl-bench
python3 $M/batch.py $KNOBS_TMP/gate7 --parallel 4 --repeat 3 --label gate-seven

python3 $M/probes.py ../ethos-oracle/fixtures/synthetic/simple-text/document.pdf $KNOBS_TMP/probes
```

The two censuses were run concurrently, so `extract_wall_s` in the engine census is indicative
only. The batch timing ran alone afterwards, and `results/batch.txt` records the load average on
either side of it.

## What each instrument measures

| Instrument | Measures | Result file |
| --- | --- | --- |
| [`corpora.py`](corpora.py) | Lists the 297 PDFs as `corpus<TAB>path`; the others import it | — |
| [`qpdf_census.py`](qpdf_census.py) | Per PDF, with qpdf: `--is-encrypted`, `--requires-password` (exit 3 = encrypted but the empty password opens it, exit 0 = a secret is required), the encryption method and `R`, `--show-npages`, and from `--json=2` the colour-space families declared in page `/Resources` (inherited resources followed), in every resource dictionary in the file, and on every image XObject, plus `/Pattern` and `/Shading` resource entries | [`results/qpdf-census.tsv`](results/qpdf-census.tsv) |
| [`engine_census.py`](engine_census.py) | Per PDF, with the engine: `classify` and `extract` exit codes and the bracketed refusal code from stderr; for each artifact, `assurance.coverage` (the six buckets), `terminal_state` and the limitation codes, read by locating the `assurance` object in the file rather than loading the artifact. `failing-page` bisects `--max-pages` to the first page a document fails on | [`results/engine-census.tsv`](results/engine-census.tsv) |
| [`batch.py`](batch.py) | Wall time of `extract` over a directory: a sequential loop against `xargs -P N -n 1`, three repetitions, exit codes tallied so a refusing pass cannot read as a fast one | [`results/batch.txt`](results/batch.txt) |
| [`synthetic_page_error.py`](synthetic_page_error.py) | Writes a three-page PDF whose page 2 executes an operator outside Table A.1 | — |
| [`probes.py`](probes.py) | Encrypts one fixture five ways with qpdf (owner password only at AES-256, RC4-128, RC4-40; user password `secret` at AES-256, RC4-128) and runs the engine on each, printing what the artifact declares and which fields differ from the unencrypted artifact; then runs the engine on the synthetic page-error file whole, under `--max-pages 1` and `2`, and through the bisection | [`results/probes.txt`](results/probes.txt) |

## What was not measured, said here rather than left silent

- **Colour operator use.** The colour census counts declared colour spaces; a document that paints
  with `rg` and declares nothing shows an empty `page_cs`. Counting operator use needs decoded
  content streams, and no instrument here decodes one.
- **ExtGState, transparency and blend modes**, which a colour reader would also have to face.
- **Encrypted documents outside these corpora.** One of 297 is encrypted. The probes supply the
  cases the corpora lack, on generated files.
- **A page-local failure in a real document.** No corpus document has one, so the page-error
  behaviour is shown on the synthetic file only.
- **Batch mode on a machine nobody else is using.** The host was shared; the speedups are quoted
  loosely for that reason and the load averages are in the result file.
- **The 0.57.0 binary.** Its readings were discarded, not compared.
