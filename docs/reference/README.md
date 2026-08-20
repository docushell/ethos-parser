# Research archive — off-tree

Two research files once sat here: `ethos-parser-expansion-memo.md` (four passes over the OSS parser
landscape, plus two passes of ethos-engine bootstrap reasoning) and
`ethos-engine-parity-checklist.md` (~70 capability rows with row ids `O*`, `A*`, `P*`, `L*`, `E*`).

**They are no longer in this repository.** The owner keeps them elsewhere, on 2026-08-19. They were
evidence, not a plan, and holding a second roadmap in-tree invited exactly the confusion this
directory's old README spent a page warning about.

**Do not restore them, and do not summarise them back in.** A second document describing where the
project is going is the failure this removes.

## Where the living authority is

| Question | Document |
| --- | --- |
| What is decided, and what is the product? | [`../00-NORTH-STAR.md`](../00-NORTH-STAR.md) |
| Can I borrow feature X from parser Y? | [`../06-STEAL-REFUSE.md`](../06-STEAL-REFUSE.md) — the **living** steal / refuse extract, self-contained |
| What ships, in what order? | [`../02-ROADMAP.md`](../02-ROADMAP.md) plus the scope / milestones pair for the version |
| What can 0.29.0 actually do? | [`../CAPABILITY.md`](../CAPABILITY.md) |

`docs/00`–`docs/15` are the authority. Nothing outside them binds.

## The one rule the archive carried, which still binds

**Every headline benchmark number in that landscape is publisher-owned.** The same tool scored
0.000 and 0.693 on tables under two publishers, differing only by invocation flags. Do not carry any
of those numbers into this repository's docs, README, or anything a reader could mistake for a
claim. That includes **0.489**: it is a published ODL-local score on a corpus this repository does
not have, the chase for it is **parked** ([`../table-gate-v1.md`](../table-gate-v1.md)), and it was
never a number this project may publish as its own.

## Also deliberately not here

`ethos-docushell-parser-plan.md` stays at `~/Desktop/Stuff/repo/`, outside this repo. It is
architecture depth for the *Ethos-in-DocuShell* framing and is superseded wherever it conflicts with
the documents above. Not moved, not authoritative here.
