# Vendored data

Data, not crates. Nothing here is fetched at build time — a build whose output depends on the
network is a build nobody can reproduce.

## What is in the tree

| Data | Where | Licence |
| --- | --- | --- |
| `WinAnsiEncoding` (Windows-1252), full 0x20–0xFF | `crates/ethos-parser-pdf/src/encoding.rs` | PDF 32000-1 Annex D — a published specification table |
| `StandardEncoding`, ASCII range | same | same |
| Glyph-name subset for `/Differences` | same | same |
| **Adobe Core-14 AFM metrics**, all 14 faces | [`afm/`](afm/) | **APAFML** — see [`afm/README.md`](afm/README.md) and [`../NOTICE`](../NOTICE) |

The encoding tables live in Rust rather than a data file because they are 256-entry lookup tables
the compiler can bake straight into the binary. A separate file would be the same bytes with a
parser in front.

The AFMs go the other way — pristine files, embedded verbatim with `include_str!` and parsed at
runtime — for a licence reason rather than an engineering one. APAFML requires that any
modification be prominently noted in the modified file, so a generated Rust table would be a
modified file carrying an obligation, while the untouched file carries none. The bytes in the
binary are the bytes in this directory.

## What is deliberately missing

Two pieces of Adobe data are not vendored. Each gap is **declared on the wire** as a named
limitation rather than left for a consumer to discover, and the two behave differently:

| Missing | What happens | Why not vendored |
| --- | --- | --- |
| **Adobe predefined CMaps** (`UniJIS-UCS2-H` and ~167 siblings) | A document naming one is **refused** | They are CJK mappings nothing in the corpus exercises. Shipping ~168 binary files no test touches means shipping data nobody has verified. When a CJK document arrives, they land with a fixture that proves they work |
| **The full Adobe Glyph List** | An unrecognised `/Differences` name **drops that run** and declares `broken-font-encoding`, naming the glyph. Only a document that decodes nothing at all is refused | Same reasoning. The run is dropped whole rather than patched with `U+FFFD`, which would put a character in the evidence that the document does not contain, and rather than spliced, which would join words the document never wrote |

Refusing and dropping-with-a-count are two different honest behaviours. Flattening them into
"fails closed" would tell a reader to expect an error where they will get an artifact.

## The row that used to be here

**Core-14 AFM widths were the third gap, and decision #22 reversed it.** The reasoning this file
carried was:

> The advance is genuinely unknown from the document alone. Reporting it as unknown is correct;
> guessing it is exactly the failure this project exists to refuse.

That is wrong for this case, and the counter-argument was already in the tree one function away.
[`fonts.rs`](../crates/ethos-parser-pdf/src/fonts.rs) says of a composite font's `/DW` that
*"Reading a normative default is reading the document, not guessing at it"*. A document naming
`/BaseFont /Helvetica` with no `/Widths` is doing exactly that: §9.6.2.2 makes the metrics **known
and merely absent**, because it permits the omission precisely on the grounds that a conforming
reader holds them.

The refusal was also **narrower than the gap it described**. It said *widths*; these fonts declare
no descriptor either, so ascent and descent were missing too — and `ink_box` needs those
**first**. The file's own description of what it refused was smaller than what it refused.

What has NOT changed is the line either side of it: supplying Helvetica's metrics for a font the
document calls `Arial` is a metric substitution, and that is still refused. See
[`afm/README.md`](afm/README.md).

## Attribution

The encoding tables are transcriptions of PDF 32000-1 (ISO 32000-1:2008) Annex D, a published
specification. No Adobe CMap resource files are in this repository, so no Adobe BSD-3-Clause notice
is currently required. [`../NOTICE`](../NOTICE) records that, and will carry the full text if those
files ever land.

The Core-14 AFMs in [`afm/`](afm/) are Adobe's own, under **APAFML**, which is not OSI-approved and
carries obligations this repository has to meet by hand rather than by tooling —
[`afm/README.md`](afm/README.md) states them and [`../NOTICE`](../NOTICE) carries the attribution.
