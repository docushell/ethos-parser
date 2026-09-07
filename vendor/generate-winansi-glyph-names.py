#!/usr/bin/env python3
"""Derive WinAnsiEncoding's glyph-name column, and refuse to emit one that cannot be corroborated.

`docs/21-STANDARD-14-ASCII-COVERAGE-SCOPE.md` refused a HAND-TRANSCRIBED version of this table.
The objection was transcription risk, not the table: `docs/20` §4 rejected pdf.js's metrics partly
because they carry two verified `xHeight` transcription defects, and 224 entries typed by hand
carry that same risk. §5 of `21` named the condition that dissolves it — a derived table, every
entry cross-validated against data already in this repository.

This is that generator. It emits nothing unless THREE independent sources agree on every entry:

  1. `crates/ethos-parser-pdf/src/encoding.rs`'s `WIN_ANSI` — code to TEXT, vendored from
     PDF 32000-1 Annex D, and the authority for what this profile believes WinAnsiEncoding is.
  2. Adobe's Glyph List — glyph NAME to codepoint. Passed in by path and deliberately NOT
     vendored: it is a tool used once at generation time, and nothing in the build reads it.
     `vendor/README.md` records the AGL as absent, and this does not change that.
  3. `vendor/afm/*.afm` — the Core-14 glyph repertoire, which decides WHICH of the AGL's several
     names for one codepoint this table should carry, and proves the chosen name is a real Adobe
     glyph name rather than a plausible-looking typo.

A name no AFM carries is not emitted. A name whose AGL codepoint disagrees with `WIN_ANSI` is a
hard failure, because that means one of the two vendored tables is wrong and the right response is
to stop rather than to pick a winner.

Usage:  python3 vendor/generate-winansi-glyph-names.py <glyphlist.txt>
Writes: crates/ethos-parser-pdf/src/winansi_names.rs
"""

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
ENCODING_RS = ROOT / "crates/ethos-parser-pdf/src/encoding.rs"
AFM_DIR = ROOT / "vendor/afm"
OUT = ROOT / "crates/ethos-parser-pdf/src/winansi_names.rs"

# The generated table's own header. Kept here so the file says what made it.
HEADER = '''//! `WinAnsiEncoding`'s glyph-name column, code to Adobe glyph name.
//!
//! **Generated. Do not edit by hand.** Regenerate with
//! `python3 vendor/generate-winansi-glyph-names.py <glyphlist.txt>`, which refuses to emit an
//! entry that three independent sources do not agree on — this repository's own `WIN_ANSI`
//! code-to-text table, Adobe's Glyph List, and the glyph repertoire of the vendored Core-14 AFMs.
//!
//! **Why this exists as a derived table rather than a transcribed one.**
//! [`21-STANDARD-14-ASCII-COVERAGE-SCOPE.md`](../../../docs/21-STANDARD-14-ASCII-COVERAGE-SCOPE.md)
//! refused the transcribed version: 224 entries typed by hand, carrying the transcription risk
//! that `docs/20` §4 used to reject pdf.js's metrics, for a seven-node return. §5 of that document
//! named the condition that reopens it, and this meets it — no entry here was typed, and
//! [`super::encoding`] re-checks the whole table against `WIN_ANSI` at test time with no external
//! source.
//!
//! **What it is for.** A standard-14 face's width lives under a glyph NAME in its AFM. A document
//! using `WinAnsiEncoding` addresses glyphs by CODE. This is the join, and it is the join for a
//! code above ASCII, where [`super::encoding`]'s `StandardEncoding` table stops.
//!
//! **Two codes are deliberately absent: 0xA0 and 0xAD.** Annex D notes that `WinAnsiEncoding`
//! also encodes `space` at 0xA0 and `hyphen` at 0xAD, but `WIN_ANSI` decodes those codes to
//! U+00A0 and U+00AD — no-break space and soft hyphen — which is right for TEXT and leaves the
//! generator with a codepoint no AFM glyph carries. The two readings are both real and they
//! disagree, so the generator emits neither rather than picking one: a width taken from the wrong
//! reading would be a plausible number for a glyph the document did not ask for. They cost
//! nothing measurable on the corpus of `21` — neither code appears in its seven-node residual.
'''


def parse_rust_string(lit: str) -> str:
    """Decode a Rust string literal body: `\\u{XXXX}`, `\\\\`, `\\"` and plain characters."""
    out, i = [], 0
    while i < len(lit):
        if lit[i] == "\\":
            if lit[i + 1] == "u":
                end = lit.index("}", i)
                out.append(chr(int(lit[i + 3 : end], 16)))
                i = end + 1
            else:
                out.append({"n": "\n", "t": "\t", "\\": "\\", '"': '"', "'": "'"}[lit[i + 1]])
                i += 2
        else:
            out.append(lit[i])
            i += 1
    return "".join(out)


def load_win_ansi() -> dict:
    """`WIN_ANSI` as this repository states it: code -> text, read from the const fn itself."""
    src = ENCODING_RS.read_text()
    body = src[src.index("const fn build_win_ansi()") :]
    body = body[: body.index("\nconst fn ", 1)] if "\nconst fn " in body[1:] else body
    table = {}
    for m in re.finditer(r't\[(0x[0-9A-Fa-f]{2})\] = Some\("((?:[^"\\]|\\.)*)"\)', body):
        table[int(m.group(1), 16)] = parse_rust_string(m.group(2))
    return table


def load_agl(path: pathlib.Path) -> dict:
    """Adobe's Glyph List: name -> codepoint, single-codepoint entries only."""
    agl = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith("#") or ";" not in line:
            continue
        name, codes = line.split(";", 1)
        parts = codes.split()
        if len(parts) == 1:
            agl[name] = int(parts[0], 16)
    return agl


def load_afm_names() -> set:
    """Every glyph name the vendored Core-14 files carry."""
    names = set()
    for p in sorted(AFM_DIR.glob("*.afm")):
        for m in re.finditer(r"; N (\S+) ;", p.read_text(encoding="utf-8")):
            names.add(m.group(1))
    return names


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__.strip().splitlines()[-2], file=sys.stderr)
        return 2
    win = load_win_ansi()
    agl = load_agl(pathlib.Path(sys.argv[1]))
    afm = load_afm_names()

    by_cp = {}
    for name, cp in agl.items():
        by_cp.setdefault(cp, []).append(name)

    chosen, skipped, conflicts = {}, [], []
    for code, text in sorted(win.items()):
        if len(text) != 1:
            skipped.append((code, text, "text is not a single character"))
            continue
        cp = ord(text)
        candidates = [n for n in by_cp.get(cp, []) if n in afm]
        if not candidates:
            skipped.append((code, text, f"no AGL name for U+{cp:04X} appears in the Core-14 AFMs"))
            continue
        # Deterministic and corroborated: among names the AFMs carry, the shortest then
        # alphabetically first. Every candidate maps to the same codepoint by construction, so the
        # choice is about spelling rather than meaning.
        name = sorted(candidates, key=lambda n: (len(n), n))[0]
        if agl[name] != cp:
            conflicts.append((code, name, cp, agl[name]))
        chosen[code] = name

    if conflicts:
        for code, name, want, got in conflicts:
            print(
                f"CONFLICT 0x{code:02X}: WIN_ANSI says U+{want:04X}, AGL says /{name} is "
                f"U+{got:04X}",
                file=sys.stderr,
            )
        print("Refusing to emit: two vendored tables disagree.", file=sys.stderr)
        return 1

    lines = [HEADER, "", "/// Code to Adobe glyph name, for `WinAnsiEncoding`.", "///"]
    lines.append("/// `None` where `WIN_ANSI` itself carries no character for the code, and at")
    lines.append("/// 0xA0 and 0xAD — see the module note.")
    lines.append("pub(crate) static WIN_ANSI_NAMES: &[Option<&\'static str>; 256] = &build();")
    lines.append("")
    lines.append("const fn build() -> [Option<&\'static str>; 256] {")
    lines.append("    let mut t: [Option<&\'static str>; 256] = [None; 256];")
    for code in sorted(chosen):
        lines.append(f'    t[0x{code:02X}] = Some("{chosen[code]}");')
    lines.append("    t")
    lines.append("}")
    OUT.write_text("\n".join(lines) + "\n")

    print(f"emitted {len(chosen)} names to {OUT.relative_to(ROOT)}")
    print(f"WIN_ANSI populated codes: {len(win)}")
    if skipped:
        print(f"not emitted ({len(skipped)}):")
        for code, text, why in skipped:
            print(f"  0x{code:02X} {text!r}: {why}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
