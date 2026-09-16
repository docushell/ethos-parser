# M2 — Two instruction sets, one artifact

**Run 2026-09-11 against `c137339`** on an Apple M4 Pro. `aarch64-apple-darwin` built natively and
`x86_64-apple-darwin` cross-built with the same pinned 1.88.0 toolchain, then **executed** through
Rosetta 2. Instrument: [`crossarch.py`](crossarch.py).

**412 comparisons over 86 documents. Zero mismatches.**

---

## 1. Why this was run, and what it is a substitute for

When this was run, plan item 6.1 — three-OS byte-identity — was `BLOCKED`, and not on a decision.
GitHub Actions could not allocate a runner on this account, and this host has no cross toolchain
for Linux or Windows: no zig, no Docker, no mingw-w64, no musl-gcc. Cross-compiling one anyway
would produce a binary nobody has ever executed, which for an engine whose product is
byte-identical reruns is an untested claim wearing an artifact's clothes.

**The runner half no longer holds.** Since the repository went public (around 2026-09-13),
push-triggered jobs run. `.github/workflows/ci.yml` now carries the operating-system axis as two
jobs, `cross-os-digests` and `cross-os-identity`: Linux, macOS and Windows, compared byte for byte.
They have not run yet, so §4's "not licensed" stands until they have.

But the claim 6.1 exists to prove is that **output does not depend on the machine**, and that
splits into two independent axes: the operating system, and the instruction set. The second needs
no runner. An arm64 Mac with Rosetta 2 can build *and run* an x86_64 binary — different register
width, different codegen, different vector unit, same compiler.

This is also the axis the design claims to have settled by construction. `docs/01-CONTRACT.md`'s
integer centipoints exist so that geometry cannot vary with a machine's floating-point unit; a
float-based parser has no equivalent guarantee and this is exactly where it would show. So a
disagreement here would be worse news than a disagreement across operating systems: it would mean
the central design decision did not do the job it was made for.

## 2. What was compared

| | comparisons |
| --- | --- |
| `extract`, `classify` — from a PDF | 172 |
| `markdown`, `html`, `ground` — from a representation | 240 |
| of which refusals (identical stderr required) | 22 |
| **mismatches** | **0** |

86 documents: the 8 committed `fixtures/gate` PDFs, 43 `fixtures/engine` fixtures, and 35 from the
Ethos conformance corpus.

A refusal is compared too. Two builds that disagree about *which* documents they refuse, or about
the message, are as non-identical as two that disagree about an artifact — so a non-zero exit is
hashed by its stderr rather than skipped.

## 3. The instrument had a bug, and the first result was an overclaim

The first version ran all four subcommands against PDFs and reported **344 comparisons, 0
mismatches**. That number was real and the headline was wrong: `markdown` and `html` take a
`DocumentRepresentation` JSON, not a PDF, so **194 of the 344 were those two subcommands correctly
refusing their input**. Nothing was projected. The test compared `extract`, `classify`, and a large
number of identical refusal messages, then called itself byte-identity across every artifact.

The corrected instrument produces the representation ONCE with the native binary and hands the same
file to both arms, so the projection subcommands measure the projection rather than re-measuring
`extract`. Refusals fell from 194 to 22 and real comparisons rose from 150 to 390.

Recorded because the failure mode is worth naming: an instrument that exercises a refusal path can
report a perfect score while testing nothing, and the score looks better — more comparisons, still
zero mismatches — exactly as it gets weaker.

## 4. What this does and does not license

**Licensed:** the engine's artifacts are identical across x86_64 and arm64 on macOS, for every
subcommand that emits canonical bytes, including which documents it refuses.

**Not licensed:** anything about Linux or Windows. Both remain unbuilt and unexecuted here. This
closes the instruction-set axis of 6.1 and leaves the operating-system axis exactly where it was —
a filesystem, a path separator, a line ending and a locale are all OS-level and none of them are
touched by this run.

`ci/release-artifacts.sh` encodes the distinction: a target is `verified` only when it was executed
on the build host and matched, and `compiled` otherwise.

## 5. Excluded, and named rather than dropped

`nist-sp-800-53Ar5` and `nist-sp-800-161r1` are excluded from the **projection** half only: their
representations are 950 MB and 272 MB, and projecting each twice per arm costs hours. Both are
still compared through `extract` and `classify`, which is where a codegen difference would surface
first — the projections consume a representation that has already been proven identical.
