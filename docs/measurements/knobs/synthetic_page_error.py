"""Write a three-page PDF whose second page carries an operator outside PDF 32000-1 Table A.1.

Usage: synthetic_page_error.py OUT.pdf

No corpus document fails on one page while the rest of it reads (`engine-census.tsv`: every
refusal is a whole-document one — encrypted, no header, a broken xref, a structure-tree cycle),
so the engine's page-error behaviour has to be shown on a file made for the purpose. Pages 1 and 3
draw one line of Helvetica text each; page 2 draws one and then executes `foo`, which the
interpreter refuses by name rather than skipping (`content.rs`, `run_inner`). Uncompressed, with a
correct cross-reference table, so nothing but the operator is wrong with it.

Run `engine_census.py failing-page` on the output to see the bisection land on page 2, and
`ethos-parser extract` on it to see the whole document refused with page 1's clean extract
discarded.
"""
import sys


def pdf():
    contents = [
        b"BT /F1 12 Tf 72 700 Td (page one) Tj ET",
        b"BT /F1 12 Tf 72 700 Td (page two) Tj ET\nfoo",
        b"BT /F1 12 Tf 72 700 Td (page three) Tj ET",
    ]
    objs = []
    objs.append(b"<< /Type /Catalog /Pages 2 0 R >>")
    objs.append(b"<< /Type /Pages /Kids [3 0 R 5 0 R 7 0 R] /Count 3 >>")
    font = b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>"
    # Objects 3..8: page, content, page, content, page, content; font is 9.
    for i, c in enumerate(contents):
        page_no = 3 + 2 * i
        objs.append(b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 9 0 R >> >> /Contents %d 0 R >>" % (page_no + 1))
        objs.append(b"<< /Length %d >>\nstream\n" % len(c) + c + b"\nendstream")
    objs.append(font)
    out = bytearray(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n")
    offsets = []
    for n, body in enumerate(objs, 1):
        offsets.append(len(out))
        out += b"%d 0 obj\n" % n + body + b"\nendobj\n"
    xref = len(out)
    out += b"xref\n0 %d\n" % (len(objs) + 1)
    out += b"0000000000 65535 f \n"
    for o in offsets:
        out += b"%010d 00000 n \n" % o
    out += b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (len(objs) + 1, xref)
    return bytes(out)


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print(__doc__)
        sys.exit(2)
    with open(sys.argv[1], "wb") as f:
        f.write(pdf())
