"""Author the engine-owned CC0 fixtures the Ethos corpus lacks.

Each is a minimal, hand-built PDF exercising exactly one behaviour:

  show-text-quote-operators  the ' and " operators, whose omission is pdf-inspector's
                             disqualifying defect (parity checklist P6)               [M3]
  horizontal-scaling-tz      Tz, which pdf-inspector does not implement anywhere      [M3]
  synthesized-space-tj       a TJ gap wide enough that a space was intended but never
                             written                                                  [M3]
  measured-ink-box           a /FontDescriptor with real ascent/descent, so ink is
                             MEASURED — the only fixture anywhere that takes that
                             branch                                                   [M3]
  absent-font-metrics        a /FontDescriptor that exists and carries NO usable ink
                             metrics, so geometry is typed-absent while the advance is
                             known — the geometry-omission path at M5                 [M5]
  broken-font-encoding       /Differences pointing at glyph names no table carries, so a
                             naive reader emits mojibake and this one drops the run  [v0.1]
  ruled-table-grid           a 3x3 grid DRAWN with `re` rectangles, one merged cell and
                             one empty cell — the ruled-table golden               [v1-S1]
  ruled-table-overlap        two rectangles claiming the same lattice face, so the
                             locator cross-check must report mismatch              [v1-S1]
  unruled-near-miss          columns that align on two rows and miss on the third, so the
                             alignment rule must refuse rather than round them
                             together                                              [v1-S2]
  both-table-rules           one painted grid AND one aligned-text grid on the same page,
                             so the artifact carries two tables under two rule ids [v1-S2]
  ruled-wins-shared-region   a painted grid whose text ALSO forms a clean alignment grid;
                             exactly one table comes out, and it is the ruled one  [v1-S2]

Deliberately standard-14 Helvetica with /Widths supplied, so advance is computable and the
Tz fixture can assert a real difference.

Regenerating is deterministic: no timestamps, no ids, no compression. Running this script twice
produces byte-identical files, so re-pinning a hash in fixtures/manifest.json is a review of an
intended change rather than of incidental churn.
"""

import pathlib
import sys

# Helvetica widths for the ASCII range, as the fixtures declare them. These are the values the
# fixture ITSELF carries in /Widths, so the extractor reads them from the document rather than
# from any built-in table. A uniform 500 keeps the arithmetic checkable by hand.
UNIFORM_WIDTH = 500
FIRST_CHAR = 32
LAST_CHAR = 126


# What kind of /FontDescriptor a fixture carries. The three values are three distinct paths
# through crates/engine-pdf/src/metrics.rs, and the difference between the last two is the whole
# point of the absent-font-metrics fixture:
#
#   None          no descriptor at all      -> resolve_font_ink returns Absent immediately
#   "metrics"     ascent/descent present    -> from_descriptor returns Measured
#   "no-metrics"  descriptor present, but no Ascent/Descent and no /FontBBox
#                                           -> from_descriptor returns None -> Absent
#
# The third is NOT reachable by simply omitting the descriptor: it proves the reader looked at a
# descriptor, found nothing usable in it, and still refused to invent a box.
DESCRIPTOR_KINDS = (None, "metrics", "no-metrics")


def build_pdf(content: str, media=(0, 0, 300, 144), descriptor=None, differences=None) -> bytes:
    # `descriptor` is one of DESCRIPTOR_KINDS. Left unannotated so this script runs on any
    # python3 a reviewer happens to have — `str | None` in a signature is evaluated at import
    # time and raises before 3.10, which would make regenerating fixtures depend on the
    # regenerator's toolchain. The assertion below is the check that annotation would have been.
    assert descriptor in DESCRIPTOR_KINDS, f"unknown descriptor kind {descriptor!r}"
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
            "/Encoding %s /FirstChar %d /LastChar %d /Widths [%s]%s >>"
            % (
                (
                    "<< /Type /Encoding /BaseEncoding /WinAnsiEncoding /Differences [%s] >>"
                    % differences
                )
                if differences
                else "/WinAnsiEncoding",
                FIRST_CHAR,
                LAST_CHAR,
                widths,
                " /FontDescriptor 6 0 R" if descriptor else "",
            )
        ).encode(),
    ]
    if descriptor == "metrics":
        # Real Helvetica ascent/descent, declared BY THE DOCUMENT so the extractor reads them
        # rather than assuming them. This is the fixture that exercises the measured-ink path;
        # every other one exercises typed absence.
        objects.append(
            b"<< /Type /FontDescriptor /FontName /Helvetica /Flags 32 "
            b"/Ascent 718 /Descent -207 /ItalicAngle 0 /StemV 88 "
            b"/FontBBox [-166 -225 1000 931] >>"
        )
    elif descriptor == "no-metrics":
        # A descriptor that is present and structurally valid and says NOTHING about ink extent:
        # no /Ascent, no /Descent, no /FontBBox, and no embedded font program to read them from.
        # Legal PDF — those keys are only required for embedded fonts — and exactly the shape a
        # reader is tempted to paper over by falling back to the font size.
        #
        # /Widths is still supplied by build_pdf, so the ADVANCE is known. That is what isolates
        # the cause: this run's geometry is absent because the font declares no ink metrics, not
        # because the reader could not work out how wide the text is.
        objects.append(
            b"<< /Type /FontDescriptor /FontName /Helvetica /Flags 32 "
            b"/ItalicAngle 0 /StemV 88 >>"
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
    # M5's geometry-omission fixture. Advance KNOWN (/Widths is supplied), ink metrics ABSENT
    # (the descriptor carries neither Ascent/Descent nor /FontBBox). The representation keeps
    # the run with its native locator and typed-absent geometry; the grounding projection omits
    # it, counts it, and declares it.
    "absent-font-metrics": "BT /F1 24 Tf 72 72 Td (No ink metrics) Tj ET",
    # v0.1 / parity checklist P10. Two runs under a font whose /Differences point at glyph names
    # no vendored table carries.
    #
    # Run 1 is plain ASCII and decodes. Run 2 is codes 200/201/202, which /Differences remaps to
    # names that resolve to nothing — so the codes cannot be decoded at all.
    #
    # **The mojibake this fixture exists to refuse**: in WinAnsiEncoding those three codes are
    # E-grave, E-acute and E-circumflex. A reader that ignored /Differences, or that fell back to
    # the base encoding when a glyph name missed, would emit "\u00c8\u00c9\u00ca" — text the
    # document does not contain, in an artifact that looks perfectly well-formed. A reader that
    # substituted U+FFFD would be no better. This engine drops the run and declares
    # `broken-font-encoding` with the count.
    "broken-font-encoding": (
        "BT /F1 24 Tf "
        "1 0 0 1 72 110 Tm (Readable) Tj "
        "1 0 0 1 72 40 Tm (\\310\\311\\312) Tj "
        "ET"
    ),
    # v1-S1's ruled-table golden. The Ethos conformance corpus contains NO path operators at all
    # — `synthetic/table-regular-grid` lays its grid out with text position alone — so the ruled
    # detector has nothing there to run on and this fixture is the golden instead.
    #
    # A 3x3 lattice, drawn as stroked `re` cell rectangles:
    #
    #     x:  40      140     240     340
    #     y160 +-------+-------+-------+
    #          | Name  |  Q1   |  Q2   |   row 0
    #     y120 +-------+-------+-------+
    #          | Alpha |  10   |(empty)|   row 1
    #      y80 +-------+-------+-------+
    #          | Beta  |      n/a      |   row 2, cols 1-2 MERGED
    #      y40 +-------+---------------+
    #
    # Two properties are deliberate. The **merged** cell in row 2 makes CellSlot occupancy do real
    # work: a cell that spans two columns must own both slots, which is the shipped Ethos
    # ODL-adapter defect (memo s16) this engine refuses to repeat. The **empty** cell at row 1
    # col 2 is the fabrication-0 case: rectangles enclosing no text must produce an empty cell,
    # never the neighbouring "10" or "Q2".
    "ruled-table-grid": (
        "1 w "
        # row 0
        "40 120 100 40 re S 140 120 100 40 re S 240 120 100 40 re S "
        # row 1
        "40 80 100 40 re S 140 80 100 40 re S 240 80 100 40 re S "
        # row 2: col 0, then one rectangle spanning columns 1 and 2
        "40 40 100 40 re S 140 40 200 40 re S "
        "BT /F1 12 Tf "
        "1 0 0 1 50 134 Tm (Name) Tj 1 0 0 1 150 134 Tm (Q1) Tj 1 0 0 1 250 134 Tm (Q2) Tj "
        "1 0 0 1 50 94 Tm (Alpha) Tj 1 0 0 1 150 94 Tm (10) Tj "
        "1 0 0 1 50 54 Tm (Beta) Tj 1 0 0 1 150 54 Tm (n/a) Tj "
        "ET"
    ),
    # v1-S1's cross-check hostile. Two rectangles overlap, so one lattice face is claimed twice
    # and the structural CellSlot cover cannot be exact. The engine must REPORT that and never
    # nudge a coordinate to make it tile.
    "ruled-table-overlap": (
        "1 w "
        "40 80 100 40 re S 140 80 100 40 re S "
        # This one starts inside the second cell rather than on its edge: it claims a face
        # another rectangle already owns.
        "40 40 200 40 re S 140 40 100 40 re S "
        "BT /F1 12 Tf "
        "1 0 0 1 50 94 Tm (A) Tj 1 0 0 1 150 94 Tm (B) Tj "
        "1 0 0 1 50 54 Tm (C) Tj "
        "ET"
    ),
    # v1-S2's NEAR MISS. Three rows, two columns, and no path operators — the shape the
    # alignment rule is built for, spoiled by five points.
    #
    #     x:  50            200
    #         Name          Score      <- aligned
    #         Alpha         10         <- aligned
    #         Beta            12       <- 205, not 200
    #
    # Five points is 500 centipoints: well past the 150 the rule folds together, and well under
    # the 1200 it requires between two real columns. So the origins imply THREE column lines,
    # two of them half a gutter apart, and the answer is no table plus a named refusal.
    #
    # The tempting alternative is to round 205 back to 200 because it is "obviously" the same
    # column. That is choosing between two alignments the document does not choose between, and
    # a reader would have no way to know a coordinate had been moved. Whether the author meant a
    # third column or fumbled the second is not knowable from the file.
    "unruled-near-miss": (
        "BT /F1 12 Tf "
        "1 0 0 1 50 160 Tm (Name) Tj 1 0 0 1 200 160 Tm (Score) Tj "
        "1 0 0 1 50 120 Tm (Alpha) Tj 1 0 0 1 200 120 Tm (10) Tj "
        "1 0 0 1 50 80 Tm (Beta) Tj 1 0 0 1 205 80 Tm (12) Tj "
        "ET"
    ),
    # v1-S2's WHICH-RULE-FIRED fixture. One page, two grids, two kinds of evidence:
    #
    #   top     a 2x2 grid the document PAINTS with `re`, text inside it   -> ruled-rects-v1
    #   bottom  a 2x2 grid implied by text alignment alone, no rectangles  -> unruled-align-v1
    #
    # The artifact must carry BOTH, each naming the rule that produced it. A single
    # `table_detection` string on the profile could never express this: the profile says which
    # rules ran, and only a per-table field can say which one found any given table.
    "both-table-rules": (
        "1 w "
        "40 260 100 40 re S 140 260 100 40 re S "
        "40 220 100 40 re S 140 220 100 40 re S "
        "BT /F1 12 Tf "
        "1 0 0 1 50 274 Tm (R1) Tj 1 0 0 1 150 274 Tm (R2) Tj "
        "1 0 0 1 50 234 Tm (R3) Tj 1 0 0 1 150 234 Tm (R4) Tj "
        "1 0 0 1 50 140 Tm (Ua) Tj 1 0 0 1 200 140 Tm (Ub) Tj "
        "1 0 0 1 50 100 Tm (Uc) Tj 1 0 0 1 200 100 Tm (Ud) Tj "
        "ET"
    ),
    # v1-S2's ARBITRATION fixture. A painted 2x2 grid whose four runs are ALSO a flawless 2x2
    # alignment — so both rules can describe this region, and exactly one of them may.
    #
    # Ruled wins. A ruling line is evidence the author left; an alignment cluster is a decision
    # this engine made, and preferring ours would be preferring our inference to their statement.
    # The two grids are never averaged either: that would produce a grid neither rule found, with
    # no rule id that honestly describes it.
    # The cell rectangles are ADJACENT (40-140 and 140-240), not spaced. A gap between them
    # would leave an uncovered lattice face, the ruled rule's coherence precondition would
    # refuse the grid, and the fixture would silently test the opposite of what it claims —
    # measured while authoring it, which is why this note is here.
    "ruled-wins-shared-region": (
        "1 w "
        "40 80 100 40 re S 140 80 100 40 re S "
        "40 40 100 40 re S 140 40 100 40 re S "
        "BT /F1 12 Tf "
        "1 0 0 1 50 94 Tm (A) Tj 1 0 0 1 200 94 Tm (B) Tj "
        "1 0 0 1 50 54 Tm (C) Tj 1 0 0 1 200 54 Tm (D) Tj "
        "ET"
    ),
}

# name -> MediaBox. The ruled fixtures need a wider page than the 300x144 default.
MEDIA = {
    "ruled-table-grid": (0, 0, 400, 200),
    "ruled-table-overlap": (0, 0, 300, 160),
    "unruled-near-miss": (0, 0, 300, 200),
    "both-table-rules": (0, 0, 320, 320),
    "ruled-wins-shared-region": (0, 0, 300, 160),
}

# name -> /Differences array body. Only the broken-encoding fixture carries one.
DIFFERENCES = {
    "broken-font-encoding": "200 /nonexistentglyphone /nonexistentglyphtwo /nonexistentglyphthree",
}

# name -> descriptor kind. Absent from this map means no descriptor at all.
DESCRIPTORS = {
    "measured-ink-box": "metrics",
    "absent-font-metrics": "no-metrics",
}


def main() -> int:
    root = pathlib.Path(sys.argv[1])
    for name, content in FIXTURES.items():
        d = root / name
        d.mkdir(parents=True, exist_ok=True)
        pdf = build_pdf(
            content,
            media=MEDIA.get(name, (0, 0, 300, 144)),
            descriptor=DESCRIPTORS.get(name),
            differences=DIFFERENCES.get(name),
        )
        (d / "document.pdf").write_bytes(pdf)
        print(f"{name}: {len(pdf)} bytes")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
