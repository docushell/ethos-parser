#!/usr/bin/env python3
"""Derive PDFDocEncoding, code to Unicode, and refuse to emit a code three decoders do not agree on.

`docs/29-OUTLINES-SCOPE.md` §4 made this table an optional slice (§9 `S-ENC`) and set its bar: it
must clear `docs/21-STANDARD-14-ASCII-COVERAGE-SCOPE.md` §5 — a DERIVED table, not a transcribed
one. The codes that matter are exactly where PDFDocEncoding, Latin-1 and Windows-1252 disagree, so
the `WIN_ANSI` table already vendored cannot corroborate them. Three independent PDF
implementations can.

This is that generator. It writes one probe PDF — an outline of 256 entries, entry i titled
`A<byte i>A` — and reads each title back through:

  1. qpdf's text-string decoder, via `qpdf --json=1`, which renders strings with `getUTF8Value`;
  2. pdf.js's outline titles, via `getOutline()` in `pdfjs-dist`'s legacy build;
  3. Ghostscript's `doc-to-ucode` table in `lib/pdf_info.ps`, read as data rather than run.

A code is emitted only where all three give the same single scalar. Every other code is `None`,
and the strict decoder refuses it. The Latin-1 range this engine already decoded is a hard check:
if the three ever stop agreeing that 0x20-0x7E and 0xA1-0xFF (less 0xAD) are Latin-1, the
generator stops rather than emitting a table that would change text decoded today.

Usage:  python3 vendor/generate-pdfdoc-encoding.py <pdfjs-dist dir> <ghostscript lib dir>
Writes: crates/ethos-parser-pdf/src/pdfdoc.rs
"""

import json
import pathlib
import re
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
OUT = ROOT / "crates/ethos-parser-pdf/src/pdfdoc.rs"
FIRST = 10  # object number of the first outline entry in the probe


def build_probe() -> bytes:
    """A one-page PDF whose outline's entry i is titled with the hex string <41 ii 41>."""
    objs = {
        1: b"<< /Type /Catalog /Pages 2 0 R /Outlines 3 0 R >>",
        2: b"<< /Type /Pages /Kids [4 0 R] /Count 1 >>",
        3: b"<< /Type /Outlines /First %d 0 R /Last %d 0 R /Count 256 >>" % (FIRST, FIRST + 255),
        4: b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> >>",
    }
    for i in range(256):
        o = FIRST + i
        parts = [b"/Title <41%02X41>" % i, b"/Parent 3 0 R", b"/Dest [4 0 R /Fit]"]
        if i > 0:
            parts.append(b"/Prev %d 0 R" % (o - 1))
        if i < 255:
            parts.append(b"/Next %d 0 R" % (o + 1))
        objs[o] = b"<< " + b" ".join(parts) + b" >>"
    out = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
    offsets = {}
    for k in sorted(objs):
        offsets[k] = len(out)
        out += b"%d 0 obj\n" % k + objs[k] + b"\nendobj\n"
    size = max(objs) + 1
    xref = len(out)
    out += b"xref\n0 %d\n0000000000 65535 f \n" % size
    for k in range(1, size):
        out += (b"%010d 00000 n \n" % offsets[k]) if k in offsets else b"0000000000 65535 f \n"
    out += b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (size, xref)
    return bytes(out)


def strip_frame(title: str, who: str, code: int) -> list:
    if not (title.startswith("A") and title.endswith("A")):
        raise SystemExit(f"{who}: entry 0x{code:02X} lost its frame: {title!r}")
    return [ord(c) for c in title[1:-1]]


def qpdf_map(probe: pathlib.Path) -> tuple:
    version = subprocess.run(["qpdf", "--version"], capture_output=True, text=True, check=True)
    objs = json.loads(
        subprocess.run(
            ["qpdf", "--json=1", "--json-key=objects", str(probe)],
            capture_output=True,
            text=True,
        ).stdout
    )["objects"]
    table = {i: strip_frame(objs[f"{FIRST + i} 0 R"]["/Title"], "qpdf", i) for i in range(256)}
    return version.stdout.split()[2], table


PDFJS_SCRIPT = """
const [pdfjsDir, probe] = process.argv.slice(1);
const { readFileSync } = await import("node:fs");
const pdfjs = await import(pdfjsDir + "/legacy/build/pdf.mjs");
const doc = await pdfjs.getDocument({ data: new Uint8Array(readFileSync(probe)), verbosity: 0 }).promise;
const outline = await doc.getOutline();
process.stdout.write(JSON.stringify(outline.map((item) => item.title)));
"""


def pdfjs_map(pdfjs_dir: pathlib.Path, probe: pathlib.Path) -> tuple:
    version = json.loads((pdfjs_dir / "package.json").read_text())["version"]
    titles = json.loads(
        subprocess.run(
            ["node", "--input-type=module", "-e", PDFJS_SCRIPT, str(pdfjs_dir.resolve()), str(probe)],
            capture_output=True,
            text=True,
            check=True,
        ).stdout
    )
    if len(titles) != 256:
        raise SystemExit(f"pdf.js read {len(titles)} outline entries, not 256")
    return version, {i: strip_frame(t, "pdf.js", i) for i, t in enumerate(titles)}


def ghostscript_map(gs_lib: pathlib.Path) -> tuple:
    """`/doc-to-ucode [ ... ] readonly def`: integers, and `start step limit {} for` runs."""
    src = (gs_lib / "pdf_info.ps").read_text(encoding="latin-1")
    body = re.search(r"/doc-to-ucode \[(.*?)\] readonly def", src, re.S).group(1)
    stack = []
    for tok in re.findall(r"16#[0-9A-Fa-f]+|\d+|\{\}|for", body):
        if tok == "{}":
            continue
        if tok == "for":
            limit, step, start = stack.pop(), stack.pop(), stack.pop()
            stack.extend(range(start, limit + 1, step))
        else:
            stack.append(int(tok[3:], 16) if tok.startswith("16#") else int(tok))
    if len(stack) != 256:
        raise SystemExit(f"Ghostscript's doc-to-ucode has {len(stack)} entries, not 256")
    # An install keeps its version as a directory beside `lib` (`share/ghostscript/10.06.0`).
    beside = [d.name for d in gs_lib.parent.iterdir() if re.fullmatch(r"\d+\.\d+\.\d+", d.name)]
    version = beside[0] if len(beside) == 1 else "(version not found beside lib)"
    return version, {i: [cp] for i, cp in enumerate(stack)}


HEADER = """//! PDFDocEncoding, code to Unicode scalar: the encoding of a PDF text string that is not
//! UTF-16BE (PDF 32000-1 §7.9.2.2, Annex D.2).
//!
//! **Generated. Do not edit by hand.** Regenerate with
//! `python3 vendor/generate-pdfdoc-encoding.py <pdfjs-dist dir> <ghostscript lib dir>`, which
//! emits a code only where three independent decoders read it as the same scalar — qpdf
//! {qpdf}'s text-string decoder, pdf.js {pdfjs}'s outline titles and Ghostscript {gs}'s
//! `doc-to-ucode` table.
//!
//! **Why derived rather than transcribed.**
//! [`21-STANDARD-14-ASCII-COVERAGE-SCOPE.md`](../../../docs/21-STANDARD-14-ASCII-COVERAGE-SCOPE.md)
//! §4 refused a hand-typed Annex D table, and §5 named the condition that reopens one: a derived
//! table, every entry cross-validated. `WIN_ANSI` cannot be the cross-check here — 0x80–0x9F is
//! exactly where it and PDFDocEncoding disagree — so three PDF implementations are.
//! [`29-OUTLINES-SCOPE.md`](../../../docs/29-OUTLINES-SCOPE.md) §9 `S-ENC` is the slice.
//!
//! **{none} codes are `None`, because the three disagree:**
{disputed}
//!
//! A `None` is refused by the strict decoders and becomes U+FFFD in the lenient one
//! (`crate::forms`).
"""


def main() -> int:
    if len(sys.argv) != 3:
        print(__doc__.strip().splitlines()[-2], file=sys.stderr)
        return 2
    pdfjs_dir, gs_lib = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    with tempfile.TemporaryDirectory() as tmp:
        probe = pathlib.Path(tmp) / "probe.pdf"
        probe.write_bytes(build_probe())
        sources = {
            "qpdf": qpdf_map(probe),
            "pdf.js": pdfjs_map(pdfjs_dir, probe),
            "Ghostscript": ghostscript_map(gs_lib),
        }

    table, disputed = {}, []
    for code in range(256):
        reads = {who: m[code] for who, (_, m) in sources.items()}
        values = list(reads.values())
        if all(v == values[0] for v in values) and len(values[0]) == 1:
            table[code] = values[0][0]
        else:
            def said(v):
                if not v:
                    return "nothing"
                if v == [0] and code != 0:
                    return "0, its mark for no character"
                return "/".join(f"U+{c:04X}" for c in v)

            shown = ", ".join(f"{who} {said(v)}" for who, v in reads.items())
            disputed.append(f"//! - `0x{code:02X}`: {shown}.")

    kept = [c for c in range(0x20, 0x7F)] + [c for c in range(0xA1, 0x100) if c != 0xAD]
    drift = [c for c in kept if table.get(c) != c]
    if drift:
        print(
            "Refusing to emit: the decoders no longer agree that these codes are Latin-1, "
            "which this engine decodes today: " + ", ".join(f"0x{c:02X}" for c in drift),
            file=sys.stderr,
        )
        return 1

    versions = {who: v for who, (v, _) in sources.items()}
    lines = [
        HEADER.format(
            qpdf=versions["qpdf"],
            pdfjs=versions["pdf.js"],
            gs=versions["Ghostscript"],
            none=len(disputed),
            disputed="\n".join(disputed),
        ).rstrip("\n"),
        "",
        "/// Code to scalar, and `None` where the three decoders disagree — see the module note.",
        "pub(crate) static PDFDOC: [Option<char>; 256] = build();",
        "",
        "const fn build() -> [Option<char>; 256] {",
        "    let mut t: [Option<char>; 256] = [None; 256];",
    ]
    code = 0
    while code < 256:
        if table.get(code) == code:
            end = code
            while end + 1 < 256 and table.get(end + 1) == end + 1:
                end += 1
            lines.append(f"    t = latin1(t, 0x{code:02X}, 0x{end:02X});")
            code = end + 1
        else:
            if code in table:
                lines.append(f"    t[0x{code:02X}] = Some('\\u{{{table[code]:04X}}}');")
            code += 1
    lines += [
        "    t",
        "}",
        "",
        "/// `first..=last`, each read as the Latin-1 scalar of the same value.",
        "const fn latin1(mut t: [Option<char>; 256], first: u8, last: u8) -> [Option<char>; 256] {",
        "    let mut code = first;",
        "    loop {",
        "        t[code as usize] = Some(code as char);",
        "        if code == last {",
        "            return t;",
        "        }",
        "        code += 1;",
        "    }",
        "}",
    ]
    OUT.write_text("\n".join(lines) + "\n")

    print(f"emitted {len(table)} codes to {OUT.relative_to(ROOT)}; {len(disputed)} disputed")
    for who, v in versions.items():
        print(f"  {who} {v}")
    for d in disputed:
        print("  " + d[4:])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
