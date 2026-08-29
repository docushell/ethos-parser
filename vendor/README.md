# Vendored data

Data, not crates. Nothing here is fetched at build time: a build whose output depends on the
network is a build whose output nobody can reproduce.

## What is carried

| Data | Where | Licence | Why in-tree |
| --- | --- | --- | --- |
| `WinAnsiEncoding` (Windows-1252), full 0x20–0xFF | `crates/ethos-parser-pdf/src/encoding.rs` | PDF 32000-1 Annex D — a specification table, authored here | Small, does not churn, and needed for every Latin document |
| `StandardEncoding`, ASCII range | same | same | Same, plus the two codes where it is *not* ASCII |
| Glyph-name subset for `/Differences` | same | same | The names the corpus uses, plus the obvious Latin set and the f-ligatures |

They live in Rust rather than in a data file because they are 256-entry lookup tables that the
compiler can bake into `.rodata`. A separate file would be the same bytes with a parser in front.

## What is deliberately **not** carried

Each of these is **declared** rather than left to be inferred, as a limitation code inside
`assurance.limitations` — `predefined-cmaps-not-vendored` for the CMaps, `font-widths-absent` for
the Core-14 widths, `broken-font-encoding` for a glyph name outside the subset. This sentence named a `not_decoded` list until v2-S13.3: that was M3's
spelling, M4 absorbed the list into the L1 gate, and no artifact has carried the field since.

**Only one of the three fails closed, and the sentence used to say all of them did.** A document
naming a predefined CMap is genuinely refused. A standard-14 font with no `/Widths` is **read** and
reports `advance: null` for the codes it could not measure. An unrecognised `/Differences` glyph
name **dropped the whole document through v0 and stopped doing so at v0.1**: the run it appears in
is dropped whole — not patched with `U+FFFD`, which would put a character in the evidence the
document does not contain, and not spliced, which would join words the document never wrote — and
the count becomes `broken-font-encoding`. Only a document that decodes *nothing* is still refused
outright. Refusing, reporting-as-unknown and dropping-and-declaring are three different behaviours
and all three are honest; flattening them into "fail closed" told a reader to expect an error where
they will get an artifact.

| Missing | Consequence | Why not |
| --- | --- | --- |
| **Adobe predefined CMaps** (`UniJIS-UCS2-H` and ~167 siblings) | A document naming one is refused | They are CJK CID mappings. Nothing in the corpus exercises them, and shipping ~168 binary files that no test touches means shipping data nobody has verified. When a CJK document arrives, they land with a fixture that proves they work |
| **Core-14 AFM widths** (Helvetica, Times, Courier, Symbol, ZapfDingbats) | A standard-14 font with no `/Widths` reports `advance: null` | The advance is genuinely unknown from the document alone. Reporting it as unknown is correct; **guessing it is the failure mode this project exists to refuse.** Origins are unaffected — they come from the content stream |
| **The full Adobe Glyph List** | An unrecognised `/Differences` name drops that run and is declared as `broken-font-encoding`; a document that decodes nothing is refused | Same reasoning. The reason string names the glyph, so the gap is actionable. This cell said *"is refused"* until v2-S13.3, which was true through v0 and changed at v0.1 — the whole-document refusal was more than the evidence required |

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
