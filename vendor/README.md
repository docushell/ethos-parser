# Vendored data

Data, not crates. Nothing here is fetched at build time: a build whose output depends on the
network is a build whose output nobody can reproduce.

## What is carried

| Data | Where | Licence | Why in-tree |
| --- | --- | --- | --- |
| `WinAnsiEncoding` (Windows-1252), full 0x20–0xFF | `crates/engine-pdf/src/encoding.rs` | PDF 32000-1 Annex D — a specification table, authored here | Small, does not churn, and needed for every Latin document |
| `StandardEncoding`, ASCII range | same | same | Same, plus the two codes where it is *not* ASCII |
| Glyph-name subset for `/Differences` | same | same | The names the corpus uses, plus the obvious Latin set and the f-ligatures |

They live in Rust rather than in a data file because they are 256-entry lookup tables that the
compiler can bake into `.rodata`. A separate file would be the same bytes with a parser in front.

## What is deliberately **not** carried

Each of these makes the engine **fail closed** with a named error, and each is declared in the
extract artifact's `not_decoded` list so a consumer sees the gap rather than inferring its absence.

| Missing | Consequence | Why not |
| --- | --- | --- |
| **Adobe predefined CMaps** (`UniJIS-UCS2-H` and ~167 siblings) | A document naming one is refused | They are CJK CID mappings. Nothing in the corpus exercises them, and shipping ~168 binary files that no test touches means shipping data nobody has verified. When a CJK document arrives, they land with a fixture that proves they work |
| **Core-14 AFM widths** (Helvetica, Times, Courier, Symbol, ZapfDingbats) | A standard-14 font with no `/Widths` reports `advance: null` | The advance is genuinely unknown from the document alone. Reporting it as unknown is correct; **guessing it is the failure mode this project exists to refuse.** Origins are unaffected — they come from the content stream |
| **The full Adobe Glyph List** | An unrecognised `/Differences` name is refused | Same reasoning. The error names the glyph, so the gap is actionable |

**This is a deviation from the milestone text**, which named "168 `.bcmap` + NOTICE". The
substance of that instruction — encoding data lives in-tree, the build never fetches, and the
Adobe attribution is reproduced — is met. The specific files are not, because they could not be
obtained and verified in this pass, and committing unverified binary data would be worse than
declaring the gap. See `docs/05-MILESTONES.md` M3.

## Attribution

The encoding tables are transcriptions of PDF 32000-1 (ISO 32000-1:2008) Annex D, a published
specification. No Adobe CMap resource files are present in this repository, so no Adobe
BSD-3-Clause notice is currently required; `../NOTICE` records that, and will carry the full text
when those files land.
