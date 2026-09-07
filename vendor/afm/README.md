# Adobe Core-14 AFM metrics

**Pristine. Nothing in this directory has been modified**, which is why no file here carries a
modification note — see the licence obligations below, where that distinction is load-bearing.

Vendored by decision **#22** of [`../../docs/00-NORTH-STAR.md`](../docs/00-NORTH-STAR.md), whose
measurement is [`20-STANDARD-14-METRICS-SCOPE.md`](../docs/20-STANDARD-14-METRICS-SCOPE.md). A PDF
may name `/BaseFont /Helvetica` with no `/Widths` and no `/FontDescriptor`, which PDF 32000-1
§9.6.2.2 permits *because* a conforming reader is expected to hold these metrics. Reading them is
reading the document; substituting them for a font the document does **not** name — Helvetica's
metrics for Arial — is not, and stays refused.

## Licence — APAFML, and it is not OSI-approved

The full text is [`MustRead.html`](MustRead.html), which **must travel with these files under that
exact filename**. Four obligations bite, and the fourth is easy to miss:

1. All copyright notices are retained. Each `.afm` carries its own `Comment Copyright (c) …
   Adobe Systems Incorporated` line, untouched.
2. The AFM files are **not distributed without `MustRead.html`**.
3. All modifications to any of these files are **prominently noted in the modified file**. Nothing
   here is modified, so nothing carries such a note. Anything generated *from* these files is a
   modified file and must carry one.
4. **The licence paragraph itself is not modified.** `MustRead.html` is therefore vendored
   byte-exact, including its original classic-Mac CR line endings — it is not reformatted, not
   re-wrapped, and not converted to LF.

`APAFML` is deliberately **absent from [`../deny.toml`](../deny.toml)**. That allowlist governs
crate licences in the resolved dependency graph; these are data files and never enter it, exactly
as `deny.toml` already records for Adobe's CMap data. An entry `cargo deny` could never match is
the "just in case" entry that file's own header forbids. The consequence is stated rather than
hidden: **a non-OSI-approved licence is present here and CI is green, because no tool in this
repository can check it.** The review is decision #22, the paragraph in [`../NOTICE`](../NOTICE),
and this file.

## Provenance, and how "pristine" was established

No copy of Adobe's original distribution is reachable today, so these were taken from a project
mirror and **corroborated against two others that have no dependency on each other**. All three
were fetched independently and compared byte-for-byte:

| Source | Path | Pinned commit |
| --- | --- | --- |
| `gettalong/hexapdf` | `data/hexapdf/afm` | `8ffc295aee8262c04f00c5db5b504ba979fe9686` |
| `yob/pdf-reader` | `lib/pdf/reader/afm` | `f19128949be68ea9e7660ac53dbda0cc6fd64a3e` |
| `UglyToad/PdfPig` | `src/UglyToad.PdfPig.Fonts/Resources/AdobeFontMetrics` | `7c0ef111eae864383550427b2cc046faa0e7279d` |

**What the comparison found.** All 14 `.afm` files are **byte-identical** between `hexapdf` and
`pdf-reader`. `PdfPig`'s copies agree too once one transformation is undone: it holds them with
`CR` replaced by `LF` rather than `CRLF` collapsed to `LF`, so every line terminator became two
bytes of `\n` — the file length is unchanged and the content is the same, but the bytes are not.
That is a modification, and it is the reason `PdfPig` was not used as the source.

**`MustRead.html` is 937 bytes in all three and the licence paragraph is identical in all three.**
They differ only in line terminators: `hexapdf` holds `CR`, the other two hold `LF`. `CR` is what a
1990s Adobe GoLive file would have carried, and converting `CR`→`LF` is the common accidental
modification while the reverse is not, so `hexapdf`'s copy is taken as the unmodified one. This is
inference from the evidence available, not a claim of provenance from Adobe.

## What is here

| File | Bytes | sha256 |
| --- | ---: | --- |
| `Courier-Bold.afm` | 15675 | `ad0150d4bedcc8877742bf94251fcec13e348dd599d4603f679d92027d1e6e99` |
| `Courier-BoldOblique.afm` | 15741 | `cb82e69ef5f6d421e8f404fe00bb0d993425aae79725331d0ebf847e94e97e92` |
| `Courier-Oblique.afm` | 15783 | `b27103b2a2ef6030c110626597e2ab47bb8279075a039ab9170facb0aa1f70e1` |
| `Courier.afm` | 15677 | `521e0d7c7521efd4be78a5a9c5398e4c67d0771e396115b0346bc4ef74ada53d` |
| `Helvetica-Bold.afm` | 72096 | `b880d96baf56d0cc059f258f60b4d764ef49b555ab9db294b959c0016dee41f2` |
| `Helvetica-BoldOblique.afm` | 72192 | `69984a35ca26973a39f261cf83e0d367ea2e6517c590b0b030d4d4a219d9c269` |
| `Helvetica-Oblique.afm` | 77443 | `b4609b71b660a392ac09df35060271a876c2ce66617dd83bf826f742bb9d9721` |
| `Helvetica.afm` | 77343 | `da33f1870474c8e68bfe3e2353ff107ab6c6eea1f9836ce2aaf1e1a07b17982f` |
| `MustRead.html` | 937 | `be71e055bb2551a6fc8b60dcf094bf40357858980abde86f76cc9a3c144d5403` |
| `Symbol.afm` | 9953 | `3d2128a820375a10de9bc8bf6cfb15ded482c01ca0f95cc0b3277f37ec8bde66` |
| `Times-Bold.afm` | 66839 | `b4a000ed85cb22c6cdd985aa0fd3f6f78ed5079b7c6860dc4f0234e0d0e3c522` |
| `Times-BoldItalic.afm` | 62026 | `93c4744ba955215de02c4aae0b777133442ada2f7ab5a2af30a040b792b3c55d` |
| `Times-Italic.afm` | 68995 | `ed37fa2e6a67b5b17dfd47f36fc7e90df32891a4408860dbc8d4cbbe9959e242` |
| `Times-Roman.afm` | 62879 | `768e1cabea085d489a63da3e80b96bc5abf0ec98d3073c9b4d6ba76e7bccba64` |
| `ZapfDingbats.afm` | 9752 | `a32565c90afd1b57a7008fc567b78d95cf1c22adff5e086094d666d88b039859` |

Symbol and ZapfDingbats declare **no `Ascender` or `Descender`** — only `FontBBox`. That is a
property of those two files, not an omission here, and decision #22 records that the fallback it
forces carries 4 nodes on the corpus that was measured.
