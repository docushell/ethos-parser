# The Adobe Glyph List

`glyphlist.txt` is Adobe's Glyph List, table version 2.0 (September 20, 2002), **unmodified**: 4,281
glyph names, each with the Unicode scalar values it stands for. It is vendored by decision #59 of
[`docs/00-NORTH-STAR.md`](../../docs/00-NORTH-STAR.md), measured in
[`docs/33-UNMAPPED-CODES-SCOPE.md`](../../docs/33-UNMAPPED-CODES-SCOPE.md) §6.

**Source.** `glyphlist.txt` of [adobe-type-tools/agl-aglfn](https://github.com/adobe-type-tools/agl-aglfn)
at commit `4036a9ca80a62f64f9de4f7321a9a045ad0ecfd6`. The bytes here are that file's text as
fontTools embeds it verbatim from the same commit (`fontTools/agl.py`, `_aglText`); two fontTools
releases carry it byte for byte alike. No network was used to vendor it.

| | |
| --- | --- |
| Bytes | 78,060 |
| sha256 | `a3b2f61ced9f3644cc0d4ecde5c59df34ca286c689d9484a43a710a81c466789` |
| Entries | 4,281, of which 192 are private-use values the engine never produces |

**Licence.** BSD-3-Clause, `Copyright 2002-2019 Adobe` — the notice at the head of the file, which
stays with it. [`../../NOTICE`](../../NOTICE) reproduces it, as the licence requires for a
distribution in binary form.

**How the engine reads it.** [`crates/ethos-parser-pdf/src/agl.rs`](../../crates/ethos-parser-pdf/src/agl.rs)
embeds the file with `include_str!` and reads it once. A glyph name the profile's own table does not
hold — a `/Differences` name, a `MacRomanEncoding` name above ASCII, a font program's `post` name —
is read as the list gives it. An entry in the private-use area (a small capital, an old-style
figure) reads as nothing, and the table's own names win where both hold one: `/fi` stays the two
letters rather than the list's ligature U+FB01.

Nothing here is modified, so nothing carries a modification note. Changing the file changes the
profile's `cmap_data_version`, which moves `profile_sha256`.
