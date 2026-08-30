# Research archive — moved out of the tree

Two research files used to live here: a survey of the open-source parser landscape, and a ~70-row
capability checklist. **They are no longer in this repository** — the owner keeps them elsewhere as
of 2026-08-19.

They were evidence, not a plan. Keeping a second document that describes where the project is going
invites exactly the confusion this directory used to spend a page warning about. **Do not restore
them, and do not summarise them back in.**

## Where the living authority is

| Question | Document |
| --- | --- |
| What is decided, and what is the product? | [`../00-NORTH-STAR.md`](../00-NORTH-STAR.md) |
| Can I borrow feature X from parser Y? | [`../06-STEAL-REFUSE.md`](../06-STEAL-REFUSE.md) |
| What ships, in what order? | [`../02-ROADMAP.md`](../02-ROADMAP.md), plus the scope and milestone pair for that version |
| What can this build actually do? | [`../CAPABILITY.md`](../CAPABILITY.md) |

## The one rule the archive carried, which still binds

**Every headline benchmark number in this space belongs to whoever published it.** The same tool has
scored 0.000 and 0.693 on tables under two different publishers, differing only by invocation flags.

Do not carry any of those numbers into this repository's docs, README, or anywhere a reader could
mistake them for a claim about this engine — including the 49% table figure, which is a published
score on a corpus this repository does not have. See [`../table-gate-v1.md`](../table-gate-v1.md).
