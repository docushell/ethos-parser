"""Author the engine-owned CC0 fixtures M3 needs and the Ethos corpus lacks.

Each is a minimal, hand-built PDF exercising exactly one behaviour:

  show-text-quote-operators  the ' and " operators, whose omission is pdf-inspector's
                             disqualifying defect (parity checklist P6)
  horizontal-scaling-tz      Tz, which pdf-inspector does not implement anywhere
  synthesized-space-tj       a TJ gap wide enough that a space was intended but never written

Deliberately standard-14 Helvetica with /Widths supplied, so advance is computable and the
Tz fixture can assert a real difference.
"""

import pathlib
import sys

# Helvetica widths for the ASCII range, as the fixtures declare them. These are the values the
# fixture ITSELF carries in /Widths, so the extractor reads them from the document rather than
# from any built-in table. A uniform 500 keeps the arithmetic checkable by hand.
UNIFORM_WIDTH = 500
FIRST_CHAR = 32
LAST_CHAR = 126


def build_pdf(content: str, media=(0, 0, 300, 144), descriptor: bool = False) -> bytes:
    widths = " ".join(str(UNIFORM_WIDTH) for _ in range(FIRST_CHAR, LAST_CHAR + 1))
    objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        (
            "<< /Type /Page /Parent 2 0 R /MediaBox [%d %d %d %d] "
            "/Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>" % media
        ).encode(),
        None,  # content stream, filled below
        (
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica "
            "/Encoding /WinAnsiEncoding /FirstChar %d /LastChar %d /Widths [%s]%s >>"
            % (FIRST_CHAR, LAST_CHAR, widths, " /FontDescriptor 6 0 R" if descriptor else "")
        ).encode(),
    ]
    if descriptor:
        # Real Helvetica ascent/descent, declared BY THE DOCUMENT so the extractor reads them
        # rather than assuming them. This is the fixture that exercises the measured-ink path;
        # every other one exercises typed absence.
        objects.append(
            b"<< /Type /FontDescriptor /FontName /Helvetica /Flags 32 "
            b"/Ascent 718 /Descent -207 /ItalicAngle 0 /StemV 88 "
            b"/FontBBox [-166 -225 1000 931] >>"
        )
    stream = content.encode()
    objects[3] = b"<< /Length %d >>\nstream\n%s\nendstream" % (len(stream), stream)

    out = bytearray(b"%PDF-1.7\n")
    offsets = [0]
    for i, body in enumerate(objects, start=1):
        offsets.append(len(out))
        out += b"%d 0 obj\n%s\nendobj\n" % (i, body)

    xref_at = len(out)
    out += b"xref\n0 %d\n" % (len(objects) + 1)
    out += b"0000000000 65535 f \n"
    for off in offsets[1:]:
        # Exactly 20 bytes per entry, per PDF 32000-1 s7.5.4 — note the trailing space before
        # the newline. The table-regular-grid fixture in the Ethos corpus gets this wrong at 19
        # bytes, which is why lopdf refuses it.
        out += b"%010d 00000 n \n" % off
    out += b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (
        len(objects) + 1,
        xref_at,
    )
    return bytes(out)


# name -> (content, wants_font_descriptor)
FIXTURES = {
    # ' moves to the next line and shows; " also sets word and char spacing first.
    # Both are omitted or mishandled by readers that only match Tj/TJ.
    "show-text-quote-operators": (
        "BT /F1 24 Tf 20 TL 72 110 Td "
        "(plain Tj) Tj "
        "(apostrophe op) ' "
        "0 0 (quote op) \" "
        "ET"
    ),
    # Two identical strings, one at Tz 100 and one at Tz 50. The advances must differ.
    # Tm is absolute; a second Td would accumulate from the line matrix and walk off the page.
    "horizontal-scaling-tz": (
        "BT /F1 24 Tf "
        "1 0 0 1 72 110 Tm 100 Tz (AAAA) Tj "
        "1 0 0 1 72 40 Tm 50 Tz (AAAA) Tj "
        "ET"
    ),
    # Carries a /FontDescriptor, so ink is MEASURED rather than typed-absent. Every other
    # fixture takes the absence path; without this one the measured branch is untested.
    "measured-ink-box": "BT /F1 24 Tf 72 72 Td (Measured) Tj ET",
    # A TJ gap of -500 thousandths: far wider than kerning, so a space was intended and never
    # written as a glyph.
    "synthesized-space-tj": (
        "BT /F1 24 Tf 72 72 Td [(one) -500 (two)] TJ ET"
    ),
}


def main() -> int:
    root = pathlib.Path(sys.argv[1])
    for name, content in FIXTURES.items():
        d = root / name
        d.mkdir(parents=True, exist_ok=True)
        pdf = build_pdf(content, descriptor=(name == "measured-ink-box"))
        (d / "document.pdf").write_bytes(pdf)
        print(f"{name}: {len(pdf)} bytes")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
