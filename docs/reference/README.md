# Research archive

These two files are the evidence base that produced the implementation docs in `docs/00`–`docs/07`.
They were written as research, not as a plan. **They are not the living roadmap.**

## What is here

| File | What it is | How to use it |
| --- | --- | --- |
| `ethos-parser-expansion-memo.md` | Four research passes over the OSS parser landscape (OpenDataLoader, Anydoc, pdf-inspector, LiteParse), plus two passes of ethos-engine bootstrap reasoning | Read **§16 → §17 → §18** for decisions. Read **§3–§5, §16.3, §18.3** for the measurements behind them |
| `ethos-engine-parity-checklist.md` | ~70 capability rows, one per competitor feature: README claim, observed evidence, TAKE / IMPROVE / REFUSE / DEFER, target version, exit criterion | Look up a specific capability before proposing it. Row IDs (`O3`, `A5`, `P6`, `L1`, `E7`) are citable |

## Reading rules

1. **`docs/00`–`docs/07` win on any conflict.** The implementation docs are the authority; these two
   are where their claims come from. If a research file says something the implementation docs do
   not, the implementation docs were written later and deliberately.
2. **Inside the memo, later passes win.** §16 supersedes §§1–15. §17 and §18 correct §16. Each pass
   carries an explicit "corrections to prior passes" table — read it before quoting an earlier
   section.
3. **`[F]` is an observed fact, `[I]` is an inference, `[R]` is a recommendation.** The memo marks
   these deliberately. A row marked `[I]` has not been measured.
4. **Every headline benchmark number in here is publisher-owned.** Memo §18.4 and checklist §0.1
   show the same tool scoring 0.576 and 0.873 on the same corpus with the same evaluator, differing
   only by invocation flags. Do not carry any of these numbers into ethos-engine's own docs, README,
   or marketing. The one number two publishers independently agree on — ODL-local's **0.489** table
   score — is the only one used as a gate, and it is used as a *floor to beat*, not a claim to make.

## Citing these files

Use section or row IDs, not page positions:

- `reference/ethos-parser-expansion-memo.md` §18.6 — the four-way steal formula
- `reference/ethos-engine-parity-checklist.md` P6 — fail closed on unknown operators
- `reference/ethos-engine-parity-checklist.md` L6 — `trailing_space_generated`

## What is deliberately not here

`ethos-docushell-parser-plan.md` stays at `~/Desktop/Stuff/repo/`, outside this repo. It is
architecture depth for the *Ethos-in-DocuShell* framing and is superseded wherever it conflicts with
memo §16–§18. Linked from `docs/00-NORTH-STAR.md`; not moved, not authoritative here.
