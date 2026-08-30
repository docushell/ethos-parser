# Vendored data

Data, not crates. Nothing here is fetched at build time — a build whose output depends on the
network is a build nobody can reproduce.

## What is in the tree

| Data | Where | Licence |
| --- | --- | --- |
| `WinAnsiEncoding` (Windows-1252), full 0x20–0xFF | `crates/ethos-parser-pdf/src/encoding.rs` | PDF 32000-1 Annex D — a published specification table |
| `StandardEncoding`, ASCII range | same | same |
| Glyph-name subset for `/Differences` | same | same |

They live in Rust rather than a data file because they are 256-entry lookup tables the compiler can
bake straight into the binary. A separate file would be the same bytes with a parser in front.

## What is deliberately missing

Three pieces of Adobe data are not vendored. Each gap is **declared on the wire** as a named
limitation rather than left for a consumer to discover, and the three behave differently:

| Missing | What happens | Why not vendored |
| --- | --- | --- |
| **Adobe predefined CMaps** (`UniJIS-UCS2-H` and ~167 siblings) | A document naming one is **refused** | They are CJK mappings nothing in the corpus exercises. Shipping ~168 binary files no test touches means shipping data nobody has verified. When a CJK document arrives, they land with a fixture that proves they work |
| **Core-14 AFM widths** (Helvetica, Times, Courier, Symbol, ZapfDingbats) | A standard-14 font with no `/Widths` **reads**, and reports `advance: null` for codes it could not measure | The advance is genuinely unknown from the document alone. Reporting it as unknown is correct; guessing it is exactly the failure this project exists to refuse. Text origins are unaffected — they come from the content stream |
| **The full Adobe Glyph List** | An unrecognised `/Differences` name **drops that run** and declares `broken-font-encoding`, naming the glyph. Only a document that decodes nothing at all is refused | Same reasoning. The run is dropped whole rather than patched with `U+FFFD`, which would put a character in the evidence that the document does not contain, and rather than spliced, which would join words the document never wrote |

Refusing, reporting-as-unknown and dropping-with-a-count are three different honest behaviours.
Flattening them into "fails closed" would tell a reader to expect an error where they will get an
artifact.

## Attribution

The encoding tables are transcriptions of PDF 32000-1 (ISO 32000-1:2008) Annex D, a published
specification. No Adobe CMap resource files are in this repository, so no Adobe BSD-3-Clause notice
is currently required. [`../NOTICE`](../NOTICE) records that, and will carry the full text if those
files ever land.
