# Adobe's CID-to-Unicode mappings

The five `*-UCS2` files are Adobe's **ToUnicode mapping resources for PDF**, **unmodified**: for
each of Adobe's character collections, the Unicode a CID stands for. They are vendored by decision
#78 of [`docs/00-NORTH-STAR.md`](../../docs/00-NORTH-STAR.md).

**Source.** The `pdf2unicode` folder of
[adobe-type-tools/mapping-resources-pdf](https://github.com/adobe-type-tools/mapping-resources-pdf)
at commit `8c278562ebac7334d842723b6508d613d6019637` (2023-01-16), fetched over HTTPS from
`raw.githubusercontent.com` on 2026-10-18 with the owner's permission, each file checked against the
git blob SHA the repository's API reports for it at that commit. `LICENSE.txt` is the repository's
licence file from the same commit.

| File | Bytes | sha256 |
| --- | ---: | --- |
| `Adobe-Japan1-UCS2` | 294,111 | `6a9693361647a37996312cc57071bb79f8c06411207be7c730a83fda1254cd82` |
| `Adobe-GB1-UCS2` | 221,211 | `368b40ec05568faf323cfbbddf23c60d566bbc88e366981eb6541b026381a3b8` |
| `Adobe-CNS1-UCS2` | 266,531 | `8375afd535e153a7e3bcf448be93e5012b26229547a20d52520e46e3c386351f` |
| `Adobe-Korea1-UCS2` | 146,807 | `45bcf869acdcec2507f75836919e589e7137b9f9dbaa8e4975747035024248ef` |
| `Adobe-KR-UCS2` | 203,264 | `6861a3208f331ecd73369c5d51eacf3b72c820fe3c87524f79c9ac1bb16830ec` |

**Licence.** BSD-3-Clause — the notice at the head of each file (`Copyright 1990-2019 Adobe` to
`Copyright 1990-2023 Adobe`, each file its own range), which stays with it, and `LICENSE.txt`.
[`../../NOTICE`](../../NOTICE) reproduces it, as the licence requires for a distribution in binary
form.

**How the engine reads them.** [`crates/ethos-parser-pdf/src/cid_unicode.rs`](../../crates/ethos-parser-pdf/src/cid_unicode.rs)
embeds each file with `include_str!` and parses it the first time a document needs it, with the
parser a document's own `/ToUnicode` goes through: each file is a CMap by its syntax, a two-byte
codespace and `bfchar`/`bfrange` entries. A composite font under `/Identity-H` draws CIDs as its
codes; where it ships no `/ToUnicode` and its descendant's `/CIDSystemInfo` names `Adobe` and one
of `Japan1`, `GB1`, `CNS1`, `Korea1` or `KR`, its characters are read through that collection's
file. Any other collection — `Adobe-Identity`, a vendor's own — stays undecodable, its runs omitted
and counted, and so does a font under `/Identity-V`, where decision #78 scoped them out: it writes
down the page (§9.7.4.3), by metrics the engine does not read, and since decision #80 its runs carry
no advance and no box.

These are not the predefined CMaps (`UniJIS-UCS2-H` and its siblings), which map character codes
to CIDs and remain unvendored: a font naming one as its `/Encoding` is still refused.

Nothing here is modified, so nothing carries a modification note. Changing a file changes the
profile's `cmap_data_version`, which moves `profile_sha256`.
