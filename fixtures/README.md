# Fixtures

**This directory holds a manifest, not fixture bytes.**

The conformance corpus lives in the Ethos tree and is used **read-only**. `manifest.json`
references each fixture by path and `sha256`. Copying would invite drift; a hash-pinned manifest
makes a fixture change a visible event in this repo.

## Resolution — two roots

| Root | Default | Override | What it holds |
| --- | --- | --- | --- |
| `conformance` | `../ethos/fixtures` | `ETHOS_FIXTURES` | The 15-fixture conformance corpus. **The M6 oracle criterion counts exactly these** |
| `benchmark` | `../ethos/benchmarks/gate-zero/corpus` | `ETHOS_BENCH_CORPUS` | Large real-world PDFs that M2's acceptance names. Not part of the oracle count |

The second root exists because M2's load-bearing acceptance tests name documents the conformance
corpus does not contain. The bounded-cost A/B needs a 492-page PDF (`nist-sp-800-53r5`); the
exit-code-0 case needs a real simple document (`irs-form-1040-2025`). Both live in Ethos's benchmark
corpus, not `fixtures/`. Keeping them in a separate root means they cannot inflate the 15.

If a root is missing or a hash does not verify, the harness **fails with a named error**. It never
skips, and it never silently uses a different file.

## Contents — 15 conformance + 3 benchmark

| Group | Root | Count | Notes |
| --- | --- | --- | --- |
| `synthetic/` | conformance | 9 | Ethos-authored, each with committed goldens |
| `failure/` | conformance | 5 | Fail-closed cases |
| `foreign/` | conformance | 1 | OpenDataLoader round trip |
| benchmark | benchmark | 3 | `nist-sp-800-53r5` (492 pp), `nist-sp-800-63b` (80 pp), `irs-form-1040-2025` (2 pp) |

Two entries are worth knowing before you debug against them:

- **`synthetic/table-regular-grid` does not open under `lopdf`.** Its xref entries are 19 bytes where
  PDF 32000-1 §7.5.4 requires exactly 20. PDFium repairs it; `lopdf` rejects it. One valid document
  in ~26 on this corpus. v0 exits 2 and **declares** the limitation. This is expected behaviour, not
  a bug to fix at v0.
- **`failure/memory-limit-simulated` is byte-identical to `synthetic/simple-text`.** Observed, not
  assumed — both hash to `f2f6ab91…`. The limit is simulated by configuration, not by a distinct
  document. A test asserting "different fixture ⇒ different bytes" would be wrong here.

## Engine-owned fixtures

The manifest marks every entry with an `owner`. Today all 15 are `ethos`.

**One exception is planned**, at M5: an **absent-font-metrics** PDF, authored here under CC0, marked
`owner: "engine"`. It exercises the geometry-omission path, and the Ethos corpus has no fixture for
that case. Adding a second engine-owned fixture needs the same justification — that the Ethos corpus
genuinely cannot cover it — not merely convenience.

**The 15-fixture oracle criterion counts only `owner: "ethos"` entries.** Engine-owned fixtures are
additional test assets and never inflate that number.
