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
  background-panel-not-a-grid  a filled background panel with three scattered bars on it: the
                             panel covers every face the bars' edges imply, so `ruled-rects-v1`
                             called it a 7x7 table with 3 cells                   [v1-S7b]
  both-table-rules           one painted grid AND one aligned-text grid on the same page,
                             so the artifact carries two tables under two rule ids [v1-S2]
  ruled-wins-shared-region   a painted grid whose text ALSO forms a clean alignment grid;
                             exactly one table comes out, and it is the ruled one  [v1-S2]

  form-field-value           a text field whose /V is a string NO Tj on the page draws, so a
                             reader that copied widget values into the text layer would be
                             visibly caught                                             [v1-S4]
  annotation-contents        a /Text annot whose /Contents must never appear as page text, plus
                             a HIDDEN (/F bit 2) annot that must still be a node        [v1-S4]
  form-orphan-widget         a widget naming a /Parent object the file does not contain, so the
                             engine must declare the break rather than repair it        [v1-S4]
  form-xfa-stub              an /AcroForm carrying /XFA, which is declared and never parsed
                                                                                        [v1-S4]

  two-column-14-lines        two columns of seven lines, written right column first
  two-column-15-lines        the SAME page with ONE line added to the left column — the pair
                             exists to prove the reading-order rule does not flip on a line
                             count, which is exactly what pdf-inspector's does at this
                             boundary                                                   [v1-S5]

  image-xobject-drawn        a 2x2 /FlateDecode image XObject PAINTED with `Do` under a real
                             `cm`, so the placement rect comes from the matrix and not from the
                             pixel count; the corpus contains NO image XObject anywhere, so
                             without this the whole image path is untested         [v1-S6]
  image-declared-not-drawn   the SAME image declared in /Resources and never drawn. Classify
                             counts a resource; extract emits a node per `Do`. Zero nodes here
                             is the right answer and this fixture is what says so   [v1-S6]
  invisible-render-mode      a string drawn under `3 Tr`. The text must be PRESENT and flagged,
                             never filtered — checklist O21, the OpenDataLoader defect [v1-S6]
  off-page-and-offset-box    a /MediaBox whose origin is NOT (0,0), plus a /CropBox, plus one
                             run outside the crop box. No document in either corpus has a
                             non-zero box origin, which is why the coordinate defect v1-S6
                             repaired went unnoticed for six slices                 [v1-S6]
  whitespace-past-the-page-edge
                             a run of SPACES at a 1pt size whose advance carries it off the right
                             edge, with a font declaring real ink metrics. Its box was a rectangle
                             around nothing, and it put 491 of nist-sp-800-53r5's 492 pages beyond
                             reach — the seal refused the document. Zero corpus fixtures had a
                             whitespace run claiming a box                          [v1-S6.2]
  crop-box-smaller-than-media
                             a /CropBox strictly inside the /MediaBox, with a font carrying REAL
                             ink metrics and text in the cropped-away margin. The only fixture
                             where a MEASURED box sits outside the crop box — which is what makes
                             it catch a page reporting one box's dimensions beside the other
                             box's coordinates                                       [v1-S6]

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


def two_column_stream(left_lines: int, right_lines: int) -> str:
    """A two-column page, written RIGHT column first, on a 400x300 media box.

    v1-S5's anti-cliff pair. pdf-inspector decides multi-column on `min_lines < 15`: fourteen
    lines on a page come out row-interleaved and fifteen come out column-major, so a one-line
    edit reorders the whole document (docs/03-V0-SCOPE.md 3.2). Seven lines per column is
    fourteen; adding one line to the left column is fifteen. A port of that rule would read the
    two files in different orders, and `gutter-columns-v1` must read them in the same one.

    The line count is chosen to sit on that boundary and for no other reason. It is not a number
    the engine knows: the rule measures the whitespace between x=40+width and x=240, which is the
    same on both sides of the pair, and a fixture with 3 or 300 lines per column would test the
    same thing less pointedly.

    Right column first, so content-stream order and reading order genuinely disagree — a fixture
    whose stream already matched the answer would pass whether or not the rule ran.
    """
    ops = ["BT /F1 12 Tf "]
    for i in range(right_lines):
        ops.append(f"1 0 0 1 240 {260 - i * 20} Tm (R{i + 1}) Tj ")
    for i in range(left_lines):
        ops.append(f"1 0 0 1 40 {260 - i * 20} Tm (L{i + 1}) Tj ")
    ops.append("ET")
    return "".join(ops)



# A 2x2 8-bit greyscale image, deflated. Four sample bytes, one per pixel.
#
# Written as a literal rather than built with zlib at generation time so that regenerating these
# fixtures cannot depend on the zlib version a reviewer happens to have — the same reasoning that
# keeps every other byte in this file fixed. `IMAGE_SAMPLES` is what it decodes to, asserted below
# rather than trusted.
IMAGE_SAMPLES = bytes([0x00, 0x55, 0xAA, 0xFF])
IMAGE_STREAM = bytes(
    [0x78, 0xDA, 0x63, 0x08, 0x5D, 0xF5, 0x1F, 0x00, 0x03, 0x56, 0x01, 0xFF]
)


def _image_object() -> bytes:
    """Object 6: the image XObject both image fixtures share."""
    return (
        b"<< /Type /XObject /Subtype /Image /Width 2 /Height 2 "
        b"/ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /FlateDecode /Length %d >>\n"
        b"stream\n%s\nendstream" % (len(IMAGE_STREAM), IMAGE_STREAM)
    )


def build_pdf(
    content: str,
    media=(0, 0, 300, 144),
    descriptor=None,
    differences=None,
    extra_objects=None,
    catalog_extra="",
    page_extra="",
    resources_extra="",
    font_subtype="Type1",
    font_extra="",
    struct_tree=False,
) -> bytes:
    # `extra_objects` is a list of object bodies appended after the fixed five, numbered from 6.
    # Object numbering here is fixed by position (1 catalog, 2 pages, 3 page, 4 contents, 5 font),
    # and keeping it that way is what makes these files readable by hand.
    #
    # `catalog_extra` and `page_extra` are raw dictionary fragments spliced into the catalog and
    # the page. A tagged fixture uses the first for /StructTreeRoot; a form fixture uses it for
    # /AcroForm and the second for /Annots. Written by the caller rather than derived, so the
    # reference numbers in a fixture are the ones a reviewer reads in its own definition.
    #
    # `resources_extra` splices INSIDE /Resources, which is where an /XObject sub-dictionary has
    # to live (v1-S6). `page_extra` cannot serve: it lands after /Contents, outside the resource
    # dictionary entirely, so an image declared through it would be invisible to a `Do`.
    #
    # `font_extra` splices into the font dictionary — where a /ToUnicode reference goes (v1-S6.1).
    #
    # `struct_tree` decides whether the catalog names object 6 as /StructTreeRoot. It is an
    # EXPLICIT flag, and it is explicit because the heuristic it replaced ("extra_objects and no
    # other extra") mis-fired twice: an image XObject and then a /ToUnicode CMap each silently
    # became a document's structure-tree root, and the second one failed with `expected type
    # Dictionary but found type Stream` rather than anything that named the real cause. Object 6
    # belongs to whichever extra a fixture declares; only a tagged fixture says it is a tree.
    #
    # Refused together with a descriptor rather than silently renumbering: a descriptor also
    # claims object 6, and a fixture where the same number means two things is a fixture nobody
    # can check by eye.
    assert not (
        extra_objects and descriptor
    ), "a /FontDescriptor and extra objects both claim object 6; give the fixture one or the other"
    # `descriptor` is one of DESCRIPTOR_KINDS. Left unannotated so this script runs on any
    # python3 a reviewer happens to have — `str | None` in a signature is evaluated at import
    # time and raises before 3.10, which would make regenerating fixtures depend on the
    # regenerator's toolchain. The assertion below is the check that annotation would have been.
    assert descriptor in DESCRIPTOR_KINDS, f"unknown descriptor kind {descriptor!r}"
    widths = " ".join(str(UNIFORM_WIDTH) for _ in range(FIRST_CHAR, LAST_CHAR + 1))
    objects = [
        (
            "<< /Type /Catalog /Pages 2 0 R%s%s >>"
            % (
                " /StructTreeRoot 6 0 R" if struct_tree else "",
                catalog_extra,
            )
        ).encode(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        (
            "<< /Type /Page /Parent 2 0 R /MediaBox [%d %d %d %d] "
            "/Resources << /Font << /F1 5 0 R >>%s >> /Contents 4 0 R%s >>"
            % (media + (resources_extra, page_extra))
        ).encode(),
        None,  # content stream, filled below
        (
            "<< /Type /Font /Subtype /%s /BaseFont /Helvetica "
            "/Encoding %s /FirstChar %d /LastChar %d /Widths [%s]%s%s >>"
            % (
                font_subtype,
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
                font_extra,
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
    for body in extra_objects or []:
        objects.append(body.encode() if isinstance(body, str) else body)

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
    # v1-S7b's PANEL. A background rectangle with scattered bars on it, which `ruled-rects-v1`
    # turned into a table and `ruled-rects-v2` refuses.
    #
    #   y180 +-----------------------------------------+  the panel, 200 x 160, painted TWICE
    #        |   [bar A]                               |
    #        |                                         |
    #        |   Panel text          and right         |  one baseline, so the alignment rule
    #        |              [bar B]                    |  sees a single row and stays silent
    #        |                              [bar C]    |
    #    y20 +-----------------------------------------+
    #
    # The bars' eight x-edges and eight y-edges cluster into a 7 x 7 lattice of 49 faces, of
    # which the bars paint three. Under `-v1` that was a table: the panel covers every face, so
    # the coherence precondition passed, and then `detect_ruled` dropped the panel again as "the
    # table's own border". One rectangle cannot be both the only evidence a face exists and not a
    # cell. Measured on the real thing — `cfpb-home-loan-toolkit` pages 22 and 23 paint a
    # 351 x 454 pt panel behind highlight bars — that confusion produced a 17 x 13 table holding
    # 12 cells on a page whose structure tree declares no table at all.
    #
    # **Painted twice on purpose.** The real page does it, and more usefully it forecloses a
    # wrong re-fix: "skip the largest rectangle" or "skip rects[0]" would both pass a
    # single-panel fixture and fail here.
    #
    # **Filled (`f`), not stroked (`S`), and it is the first engine fixture that is.** Every
    # other ruled fixture strokes its geometry, so the fill arm feeding the lattice had no
    # fixture behind it — which is part of why this went unnoticed for six slices.
    #
    # The two runs share one baseline deliberately: with a single row line the alignment rule
    # produces neither a table nor a refusal, so this fixture tests the ruled rule alone. Text is
    # present so a test can also assert that refusing the grid costs the page none of its words.
    "background-panel-not-a-grid": (
        "0.9 g "
        "20 20 200 160 re f "
        "20 20 200 160 re f "
        "0 g "
        "40 150 40 10 re f "
        "100 90 40 10 re f "
        "160 40 40 10 re f "
        "BT /F1 12 Tf "
        "1 0 0 1 40 120 Tm (Panel text) Tj 1 0 0 1 140 120 Tm (and right) Tj "
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
    # v1-S3's golden. Four runs, one per structural-locator state — see STRUCTURE above.
    "tagged-structure-roles": (
        "BT /F1 12 Tf "
        "/P <</MCID 0>> BDC 1 0 0 1 40 110 Tm (First paragraph) Tj EMC "
        "/P <</MCID 1>> BDC 1 0 0 1 40 90 Tm (Second paragraph) Tj EMC "
        "/P <</MCID 5>> BDC 1 0 0 1 40 70 Tm (Marked but unclaimed) Tj EMC "
        "/Artifact BMC 1 0 0 1 40 50 Tm (Running head) Tj EMC "
        "1 0 0 1 40 30 Tm (Never marked) Tj "
        "ET"
    ),
    # v1-S3's /RoleMap fixture. MCID 0 is /Para, which the document maps to /P; MCID 1 is /Odd,
    # which it maps to nothing and which therefore stays /Odd.
    "tagged-rolemap": (
        "BT /F1 12 Tf "
        "/Para <</MCID 0>> BDC 1 0 0 1 40 100 Tm (Mapped to P) Tj EMC "
        "/Odd <</MCID 1>> BDC 1 0 0 1 40 70 Tm (Not mapped) Tj EMC "
        "ET"
    ),
    # v1-S3's agreeing pair: a painted 2x2 grid whose four cells are also tagged /TD.
    "tagged-table-agrees": (
        "1 w "
        "40 80 100 40 re S 140 80 100 40 re S "
        "40 40 100 40 re S 140 40 100 40 re S "
        "BT /F1 12 Tf "
        "/TD <</MCID 0>> BDC 1 0 0 1 50 94 Tm (A) Tj EMC "
        "/TD <</MCID 1>> BDC 1 0 0 1 150 94 Tm (B) Tj EMC "
        "/TD <</MCID 2>> BDC 1 0 0 1 50 54 Tm (C) Tj EMC "
        "/TD <</MCID 3>> BDC 1 0 0 1 150 54 Tm (D) Tj EMC "
        "ET"
    ),
    # v1-S3's disagreeing pair. Same painted 2x2 grid and same four marked cells; the tree claims
    # a third row whose content the page never wrote.
    "tagged-table-disagrees": (
        "1 w "
        "40 80 100 40 re S 140 80 100 40 re S "
        "40 40 100 40 re S 140 40 100 40 re S "
        "BT /F1 12 Tf "
        "/TD <</MCID 0>> BDC 1 0 0 1 50 94 Tm (A) Tj EMC "
        "/TD <</MCID 1>> BDC 1 0 0 1 150 94 Tm (B) Tj EMC "
        "/TD <</MCID 2>> BDC 1 0 0 1 50 54 Tm (C) Tj EMC "
        "/TD <</MCID 3>> BDC 1 0 0 1 150 54 Tm (D) Tj EMC "
        "ET"
    ),
    # v1-S4. A printed LABEL beside the field, and nothing else. The label is page text and stays
    # a run; the field's value is in the dictionary and must never join it.
    "form-field-value": (
        "BT /F1 12 Tf 1 0 0 1 40 120 Tm (Applicant name:) Tj ET"
    ),
    # v1-S4. Ordinary page text. Neither annotation's /Contents may appear among the runs.
    "annotation-contents": (
        "BT /F1 12 Tf 1 0 0 1 40 120 Tm (The figures below are provisional.) Tj ET"
    ),
    "form-orphan-widget": (
        "BT /F1 12 Tf 1 0 0 1 40 120 Tm (Line item:) Tj ET"
    ),
    "form-xfa-stub": (
        "BT /F1 12 Tf 1 0 0 1 40 120 Tm (Dynamic form.) Tj ET"
    ),
    # v1-S3's non-terminating tree. The text is ordinary; the structure is not.
    # v1-S6's IMAGE golden. The image is 2x2 samples and is painted into a 120x60 point
    # rectangle at (40, 60) by the `cm` — so the placement rect and the pixel count are
    # unmistakably different numbers, which is the confusion the locator exists to prevent.
    # `q`/`Q` bracket it so the matrix does not leak into anything after it.
    "image-xobject-drawn": (
        "q 120 0 0 60 40 60 cm /Im1 Do Q "
        "BT /F1 12 Tf 1 0 0 1 40 30 Tm (Below the image) Tj ET"
    ),
    # The SAME image, declared and never drawn. `Do` is what makes a node; a resource nobody
    # painted is a resource, and zero image nodes is the correct answer.
    "image-declared-not-drawn": "BT /F1 12 Tf 1 0 0 1 40 60 Tm (No Do here) Tj ET",
    # v1-S6's HIDDEN-TEXT golden, and the whole of checklist O21 in one page. The second string
    # is drawn under `3 Tr` — invisible on screen, perfectly legible to anything reading the text
    # layer. It must come out of the engine PRESENT and FLAGGED. A reader that filtered it would
    # return a page that looks clean, which is the OpenDataLoader defect.
    "invisible-render-mode": (
        "BT /F1 12 Tf 1 0 0 1 40 100 Tm (Visible sentence) Tj "
        "3 Tr 1 0 0 1 40 70 Tm (Hidden instruction) Tj "
        "0 Tr 1 0 0 1 40 40 Tm (Visible again) Tj ET"
    ),
    # v1-S6.2's golden. A run of SPACES whose advance runs off the right edge of a 300pt page,
    # in a font that declares real ascent/descent — so before v1-S6.2 the reader built an ink box
    # for it out of the font envelope and the advance, a rectangle around nothing, and `seal`
    # refused the whole document because that rectangle left the page.
    #
    # 28 spaces at 12pt with the uniform 500/1000 width is 168pt of advance from x=200, ending at
    # x=368 on a 300pt page. The visible run after it is there so the fixture also proves the
    # opposite half: text that DOES draw ink still gets its box.
    "whitespace-past-the-page-edge": (
        "BT /F1 12 Tf 1 0 0 1 200 100 Tm (                            ) Tj "
        "1 0 0 1 40 60 Tm (Visible) Tj ET"
    ),
    # v1-S6.1's golden, and the shape the corpus never had. A TrueType font — SIMPLE, so its codes
    # are one byte by PDF 32000-1 9.6 — carrying a /ToUnicode whose codespace declares TWO. Split
    # correctly this reads "Hi there"; split by the /ToUnicode codespace the codes fuse into pairs,
    # none of them is in the map, and the run is dropped entirely while the artifact blames the
    # document's encoding. That is what happened to 8,417 runs of cfpb-home-loan-toolkit.
    "simple-font-two-byte-tounicode": "BT /F1 12 Tf 1 0 0 1 40 100 Tm (Hi there) Tj ET",
    # v1-S6's TWO-FRAME golden. /CropBox [50 50 250 150] is strictly inside /MediaBox
    # [0 0 300 200], and the text sits at y=180 — inside the media box, in the margin the crop
    # box removes. The font carries real ascent/descent (see DESCRIPTORS below), so its ink box is
    # MEASURED rather than typed-absent, and that is the whole point: `seal` refuses an artifact
    # whose measured box falls outside its declared page, so a page reporting the crop box's
    # dimensions beside media-box coordinates stops producing an artifact at all.
    #
    # Every other fixture here supplies no ink metrics, so no measured box exists to fall out of
    # any page — which is exactly why nothing caught this until a probe was built by hand.
    "crop-box-smaller-than-media": "BT /F1 24 Tf 1 0 0 1 60 180 Tm (Near the top) Tj ET",
    # v1-S6's OFF-PAGE golden, which is also the coordinate-repair golden.
    #
    # /MediaBox is [0 20 300 220] and /CropBox is [0 40 300 200], so:
    #   * the box origin is NOT (0, 0) — the case that hid the discarded-origin defect, since not
    #     one document in either corpus has such a box;
    #   * the visible box is strictly smaller than the media box, so "outside the page" has a
    #     meaning that differs depending on which box you measure against.
    #
    # The first run sits inside the crop box. The second sits below it — still inside the media
    # box, so a reader measuring against /MediaBox alone would call it on-page. It is not: a
    # viewer does not show it.
    "off-page-and-offset-box": (
        "BT /F1 12 Tf 1 0 0 1 40 120 Tm (Inside the crop box) Tj "
        "1 0 0 1 40 30 Tm (Below the crop box) Tj ET"
    ),
    # v1-S5's ANTI-CLIFF pair. Identical but for one line, and they must read identically.
    # See `two_column_stream` for why fourteen and fifteen are the two numbers.
    "two-column-14-lines": two_column_stream(7, 7),
    "two-column-15-lines": two_column_stream(8, 7),
    "tagged-cycle": (
        "BT /F1 12 Tf 1 0 0 1 40 100 Tm (Text under a cyclic tree) Tj ET"
    ),
}

# v1-S3. Structure-tree objects, numbered from 6 (see build_pdf). The first entry is always the
# /StructTreeRoot, because that is the number the catalog names.
#
# Written out by hand rather than generated, so a reviewer can read the tree and the content
# stream side by side and check the (page, mcid) join by eye — which is the whole property these
# fixtures exist to pin.
STRUCTURE = {
    # The four states a structural locator can be in, one run each. A tagged document is not
    # uniformly tagged, and every one of these four is a different fact:
    #
    #   MCID 0  cited by the tree            -> pdf_tagged, role path Document/P
    #   MCID 5  marked, cited by nothing     -> pdf_mcid, and a counted gap
    #   /Artifact BMC                        -> pdf_artifact, kept in nodes, not body text
    #   no BDC at all                        -> absent, because the page marked nothing
    #
    # A reader that collapsed any two of these would lose something real: "outside the tree",
    # "not marked", and "marked as furniture" are three different statements about one page.
    "tagged-structure-roles": [
        "<< /Type /StructTreeRoot /K 7 0 R >>",
        "<< /Type /StructElem /S /Document /P 6 0 R /K [8 0 R 9 0 R] >>",
        "<< /Type /StructElem /S /P /P 7 0 R /Pg 3 0 R /K 0 >>",
        "<< /Type /StructElem /S /P /P 7 0 R /Pg 3 0 R /K 1 >>",
    ],
    # A /RoleMap: the document telling us what its own custom type means. /Para maps to /P and
    # /Odd maps to nothing, so one is understood and the other is emitted as itself. Guessing
    # that /Odd means /P because it sits where a paragraph would is the inference this refuses.
    "tagged-rolemap": [
        "<< /Type /StructTreeRoot /K 7 0 R /RoleMap << /Para /P >> >>",
        "<< /Type /StructElem /S /Document /P 6 0 R /K [8 0 R 9 0 R] >>",
        "<< /Type /StructElem /S /Para /P 7 0 R /Pg 3 0 R /K 0 >>",
        "<< /Type /StructElem /S /Odd /P 7 0 R /Pg 3 0 R /K 1 >>",
    ],
    # A tagged 2x2 table over a painted 2x2 grid: the tree and the geometry agree, so the
    # tagged-vs-geometric check is `ok`. The two derivations share no input — the tree walk never
    # reads a box and the detector never reads /S — which is what makes agreement mean something.
    "tagged-table-agrees": [
        "<< /Type /StructTreeRoot /K 7 0 R >>",
        "<< /Type /StructElem /S /Document /P 6 0 R /K [8 0 R] >>",
        "<< /Type /StructElem /S /Table /P 7 0 R /K [9 0 R 12 0 R] >>",
        "<< /Type /StructElem /S /TR /P 8 0 R /K [10 0 R 11 0 R] >>",
        "<< /Type /StructElem /S /TD /P 9 0 R /Pg 3 0 R /K 0 >>",
        "<< /Type /StructElem /S /TD /P 9 0 R /Pg 3 0 R /K 1 >>",
        "<< /Type /StructElem /S /TR /P 8 0 R /K [13 0 R 14 0 R] >>",
        "<< /Type /StructElem /S /TD /P 12 0 R /Pg 3 0 R /K 2 >>",
        "<< /Type /StructElem /S /TD /P 12 0 R /Pg 3 0 R /K 3 >>",
    ],
    # The hostile pair. The page paints a 2x2 grid and marks four cells; the TREE claims THREE
    # rows, the third citing marked content that the content stream never wrote.
    #
    # Two findings, and neither is repaired: the tagged and geometric derivations disagree about
    # the row count, and two cited content items have no run. The tempting fix is to trust one
    # side — emit the tree's 3x2, or quietly ignore the extra row — and both would be this engine
    # choosing which of two disagreeing sources to believe, with nothing on the wire to say it did.
    "tagged-table-disagrees": [
        "<< /Type /StructTreeRoot /K 7 0 R >>",
        "<< /Type /StructElem /S /Document /P 6 0 R /K [8 0 R] >>",
        "<< /Type /StructElem /S /Table /P 7 0 R /K [9 0 R 12 0 R 15 0 R] >>",
        "<< /Type /StructElem /S /TR /P 8 0 R /K [10 0 R 11 0 R] >>",
        "<< /Type /StructElem /S /TD /P 9 0 R /Pg 3 0 R /K 0 >>",
        "<< /Type /StructElem /S /TD /P 9 0 R /Pg 3 0 R /K 1 >>",
        "<< /Type /StructElem /S /TR /P 8 0 R /K [13 0 R 14 0 R] >>",
        "<< /Type /StructElem /S /TD /P 12 0 R /Pg 3 0 R /K 2 >>",
        "<< /Type /StructElem /S /TD /P 12 0 R /Pg 3 0 R /K 3 >>",
        "<< /Type /StructElem /S /TR /P 8 0 R /K [16 0 R 17 0 R] >>",
        "<< /Type /StructElem /S /TD /P 15 0 R /Pg 3 0 R /K 4 >>",
        "<< /Type /StructElem /S /TD /P 15 0 R /Pg 3 0 R /K 5 >>",
    ],
    # `/K` pointing back at an ancestor. Walking it does not terminate, and stopping partway would
    # report a structure the document does not have — so it is refused by name.
    "tagged-cycle": [
        "<< /Type /StructTreeRoot /K 7 0 R >>",
        "<< /Type /StructElem /S /Document /P 6 0 R /K [8 0 R] >>",
        "<< /Type /StructElem /S /Sect /P 7 0 R /Pg 3 0 R /K [7 0 R] >>",
    ],
}


# v1-S4. Form and annotation objects, numbered from 6 like the structure ones.
#
# The values here are deliberately strings NO `Tj` on the page draws. That is the whole test: if
# this engine ever copied a widget's value or an annotation's comment into the text layer, the
# string would turn up in a `text_run` and the assertion would catch it. A fixture whose field
# value also appeared in its page content could not tell the two apart.
# The /ToUnicode CMap for the v1-S6.1 fixture, as object 6.
#
# The codespace is <0000> <FFFF> — TWO bytes — while the font is a TrueType, whose codes are ONE
# byte by PDF 32000-1 9.6. That combination is legal, common in real documents, and present in
# ZERO fixtures before v1-S6.1. It is what a producer writes when it emits the same CMap shape for
# every font it embeds, and it is exactly the shape that made this reader fuse single-byte codes
# into pairs for six slices.
#
# The bfchar entries map the single-byte codes for "Hi there" to themselves. A reader splitting
# correctly finds every one of them; a reader splitting in pairs looks up 0x4869 ("Hi" fused),
# finds nothing, and drops the whole run.
_TOUNICODE_CMAP = (
    b"/CIDInit /ProcSet findresource begin\n"
    b"12 dict begin\nbegincmap\n"
    b"1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n"
    b"8 beginbfchar\n"
    b"<0048> <0048>\n"  # H
    b"<0069> <0069>\n"  # i
    b"<0020> <0020>\n"  # space
    b"<0074> <0074>\n"  # t
    b"<0068> <0068>\n"  # h
    b"<0065> <0065>\n"  # e
    b"<0072> <0072>\n"  # r
    b"<0061> <0061>\n"  # a
    b"endbfchar\n"
    b"endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend"
)


def _tounicode_object() -> bytes:
    return b"<< /Length %d >>\nstream\n%s\nendstream" % (
        len(_TOUNICODE_CMAP),
        _TOUNICODE_CMAP,
    )


# name -> the /ToUnicode object. Object 6, like every other extra.
TOUNICODE_OBJECTS = {
    "simple-font-two-byte-tounicode": [_tounicode_object()],
}

# name -> the font's /Subtype. Everything else is the Type1 default.
FONT_SUBTYPE = {
    "simple-font-two-byte-tounicode": "TrueType",
}

# name -> extra keys spliced into the font dictionary.
FONT_EXTRA = {
    "simple-font-two-byte-tounicode": " /ToUnicode 6 0 R",
}

# name -> extra object bodies for the image fixtures. Object 6, like every other extra.
IMAGE_OBJECTS = {
    "image-xobject-drawn": [_image_object()],
    "image-declared-not-drawn": [_image_object()],
}

FORM_OBJECTS = {
    # A single text field. Its widget IS the field — one dictionary carrying both `/FT` and
    # `/Subtype /Widget`, which is how a one-widget field is normally written — so it must produce
    # exactly ONE node, not a field plus a clone annotation.
    "form-field-value": [
        "<< /Fields [7 0 R] >>",
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T (applicant_name) "
        "/V (Wendell Ashcroft-Byrne) /Ff 2 /Rect [40 90 260 112] /P 3 0 R >>",
    ],
    # Two annotations that are not widgets. The second is HIDDEN (`/F 2`): it must still be a
    # node, flagged — deleting it because the document asked a viewer not to draw it would be an
    # edit this engine made silently (checklist O21).
    "annotation-contents": [
        "<< /Type /Annot /Subtype /Text /T (Reviewer) /NM (note-1) "
        "/Contents (Check this figure against the appendix) /Rect [40 90 60 110] >>",
        "<< /Type /Annot /Subtype /Text /F 2 "
        "/Contents (Withheld pending legal review) /Rect [40 40 60 60] >>",
    ],
    # A widget naming object 9, which this file does not contain. LiteParse repairs this in
    # memory; here the widget is emitted with what it declares about ITSELF and the broken link is
    # declared. Note the field name is therefore incomplete — visibly so, rather than papered over.
    "form-orphan-widget": [
        "<< /Fields [7 0 R] >>",
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T (line_item) "
        "/V (Orphaned value) /Parent 9 0 R /Rect [40 90 260 112] /P 3 0 R >>",
    ],
    # An /AcroForm carrying /XFA. The static field beside it still reads; the packet does not.
    "form-xfa-stub": [
        "<< /Fields [7 0 R] /XFA [(preamble) 8 0 R] >>",
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T (static_sibling) "
        "/V (Static AcroForm value) /Rect [40 90 260 112] /P 3 0 R >>",
        "<< /Length 9 >>\nstream\n<xdp:xdp>\nendstream",
    ],
}

# name -> raw fragment spliced into the catalog dictionary.
CATALOG_EXTRA = {
    "form-field-value": " /AcroForm 6 0 R",
    "form-orphan-widget": " /AcroForm 6 0 R",
    "form-xfa-stub": " /AcroForm 6 0 R",
}

# name -> raw fragment spliced into the page dictionary.
PAGE_EXTRA = {
    "form-field-value": " /Annots [7 0 R]",
    "annotation-contents": " /Annots [6 0 R 7 0 R]",
    "form-orphan-widget": " /Annots [7 0 R]",
    "form-xfa-stub": " /Annots [7 0 R]",
    # v1-S6. The crop box the off-page finding is measured against.
    "off-page-and-offset-box": " /CropBox [0 40 300 200]",
    "crop-box-smaller-than-media": " /CropBox [50 50 250 150]",
}


# name -> MediaBox. The ruled fixtures need a wider page than the 300x144 default.
MEDIA = {
    "ruled-table-grid": (0, 0, 400, 200),
    "ruled-table-overlap": (0, 0, 300, 160),
    "unruled-near-miss": (0, 0, 300, 200),
    "background-panel-not-a-grid": (0, 0, 240, 200),
    "tagged-structure-roles": (0, 0, 300, 160),
    "tagged-table-agrees": (0, 0, 300, 160),
    "tagged-table-disagrees": (0, 0, 300, 160),
    "both-table-rules": (0, 0, 320, 320),
    "ruled-wins-shared-region": (0, 0, 300, 160),
    # Wide enough for a real gutter (x=40 and x=240) and tall enough for the fifteenth line.
    "two-column-14-lines": (0, 0, 400, 300),
    "two-column-15-lines": (0, 0, 400, 300),
    "image-xobject-drawn": (0, 0, 300, 200),
    "image-declared-not-drawn": (0, 0, 300, 200),
    "invisible-render-mode": (0, 0, 300, 144),
    # **A box whose origin is not (0, 0)** — the case no document in either corpus has, and
    # therefore the case that hid a coordinate defect through six slices. Every y here is offset
    # by 20 points from the naive reading.
    "off-page-and-offset-box": (0, 20, 300, 220),
    "crop-box-smaller-than-media": (0, 0, 300, 200),
    "simple-font-two-byte-tounicode": (0, 0, 300, 144),
    "whitespace-past-the-page-edge": (0, 0, 300, 144),
}

# name -> /Resources fragment. Only the image fixtures declare an /XObject.
RESOURCES_EXTRA = {
    "image-xobject-drawn": " /XObject << /Im1 6 0 R >>",
    "image-declared-not-drawn": " /XObject << /Im1 6 0 R >>",
}

# name -> /Differences array body. Only the broken-encoding fixture carries one.
DIFFERENCES = {
    "broken-font-encoding": "200 /nonexistentglyphone /nonexistentglyphtwo /nonexistentglyphthree",
}

# name -> descriptor kind. Absent from this map means no descriptor at all.
DESCRIPTORS = {
    "measured-ink-box": "metrics",
    "absent-font-metrics": "no-metrics",
    # Real metrics, so the ink box is MEASURED — without this the fixture proves nothing, because
    # a typed-absent box can never fall outside a page.
    "crop-box-smaller-than-media": "metrics",
    # Real metrics, so an ink box would be built. Without them the run takes the typed-absence
    # path for a different reason and the fixture proves nothing.
    "whitespace-past-the-page-edge": "metrics",
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
            extra_objects=(
                STRUCTURE.get(name)
                or FORM_OBJECTS.get(name)
                or IMAGE_OBJECTS.get(name)
                or TOUNICODE_OBJECTS.get(name)
            ),
            font_subtype=FONT_SUBTYPE.get(name, "Type1"),
            font_extra=FONT_EXTRA.get(name, ""),
            struct_tree=name in STRUCTURE,
            catalog_extra=CATALOG_EXTRA.get(name, ""),
            page_extra=PAGE_EXTRA.get(name, ""),
            resources_extra=RESOURCES_EXTRA.get(name, ""),
        )
        (d / "document.pdf").write_bytes(pdf)
        print(f"{name}: {len(pdf)} bytes")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
