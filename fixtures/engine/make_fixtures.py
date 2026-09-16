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
                             metrics, on a face (`ArialMT`) that is NOT one of the standard
                             14 — so geometry is typed-absent while the advance is known.
                             The face matters since decision #22: Helvetica's metrics are
                             now readable from vendor/afm/ whatever the descriptor omits
                             — the geometry-omission path at M5                       [M5]
  absent-font-widths         /BaseFont /ArialMT with NO /Widths and NO /FontDescriptor, so
                             the advance AND the ink box are both unknown and
                             `font-widths-absent` is declared. Not Helvetica, deliberately:
                             since decision #22 a standard-14 face is answered from
                             vendor/afm/ whatever the document omits                  [#22]
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
  markdown-two-blocks        TWO runs in a font with real ink metrics, so both reach the
                             grounding artifact. The Markdown joins them with a blank line, and
                             a quote spanning that join is text the page never drew — the
                             Anchor Map golden                                    [v1.1-S1]
  untagged-shredded-line     FOUR runs on ONE baseline in a font with real ink metrics and NO
                             structure tree: three abutting exactly (`Yar`+`ro`+`w`) and a
                             fourth 40 points to the right. The only fixture whose runs share a
                             baseline, so the only one that can observe the undeclared join —
                             every other engine fixture stacks its runs and is blind to it
                                                                                  [v2.2-S5]
  markdown-hyphen-break      TWO runs 30 points apart in a font with real ink metrics, the
                             first ending `recalcu-`. The export closes the word up and the
                             joined sentence is on no page — the hyphen golden     [v1.1-S3]
  markdown-table-cells       a stroked 3x3 whose font declares real ink metrics, so its CELL
                             text reaches the grounding artifact. One merged cell, one pipe
                             inside a cell string, and a wholly empty last row — the GFM
                             golden                                               [v1.1-S2]
  tagged-list-items          an /L / /LI / /Lbl / /LBody tree with a nested /L and one item
                             whose body is two marked runs. The only list any corpus here
                             contains                                             [v1.1-S2]
  stroke-ruled-worksheet     a grid drawn as two-point STROKED rules, shaped like
                             cfpb-home-loan-toolkit page 13: one baseline rules three cells
                             instead of four (a blank cell), the three interior column rules
                             are stroked and the outer two are not. Five baselines bound FOUR
                             rows, so it also pins the undrawn top edge          [v1-S8]
  stroke-ruled-columns-not-drawn
                             the SAME horizontal ink with NO vertical rules. The band and its
                             column lines build identically and must be refused: rules ending
                             at a common x are not a boundary the author drew    [v1-S8]
  stroke-ruled-field-boxes   a stroked 2x2 whose four faces are also four widget /Rects, so the
                             boxes are the form's own and not a table's — the 1040 principle
                             without the tax form                                [v1-S8]

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
  composite-font-cid-widths  a /Type0 font whose widths live on its DESCENDANT CIDFont as /W and
                             /DW, in BOTH forms the spec defines. Neither owned corpus held a
                             single CIDFont, so the composite-width path was tested by nothing and
                             `load_widths` read `/Widths` off a dictionary the format never puts
                             it on — 50 of 50 corpus documents wrongly said "no widths" [v2.2-S3]
  composite-font-non-identity-cmap
                             the SAME descendant under a CMap this profile does not parse. /W is
                             keyed by CID; without the CMap the code is not the CID, so the width
                             must be ABSENT — and absent for that reason, not the standard-14 one
                                                                                        [v2.2-S3]
  form-xobject-text-drawn    the THIRD member of that pair: a /Subtype /Form XObject drawing text,
                             painted with the same `Do`. It is neither an image node nor a text
                             node — this profile does not descend — so the only thing that can say
                             the text existed is a COUNT, and before v2.2-S2 there was none. The
                             `Do` arm returned `None` and discarded the placement in silence
                                                                                    [v2.2-S2]
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
  ink-past-the-media-box     a run with REAL ink metrics drawn at negative x, so its measured box
                             is outside the /MediaBox — not the crop box, and not around
                             whitespace. The one case the three fixtures below did not cover, and
                             the one six DP-Bench documents met in the wild        [D4-S5]
  crop-box-smaller-than-media
                             a /CropBox strictly inside the /MediaBox, with a font carrying REAL
                             ink metrics and text in the cropped-away margin. The only fixture
                             where a MEASURED box sits outside the crop box — which is what makes
                             it catch a page reporting one box's dimensions beside the other
                             box's coordinates                                       [v1-S6]
  rotated-and-mirrored-text  seven runs with REAL ink metrics, each turned a different way: upright,
                             a quarter turn each way, upside down and mirrored by the text
                             matrix, 45 degrees, and a quarter turn by the CTM. The only engine
                             fixture whose text does not run along +x — so the only one that sees
                             a box built from the advance's x alone, which typed the first four
                             `no_ink_to_measure` and laid the last along x (docs/22 §9) [0.58.0]
  leading-gap-two-blocks     SIX single-run lines in one column with REAL ink metrics: three at a
                             14 pt leading, a 28 pt gap, three more at 14 pt. The band's modal
                             leading is 1400 centipoints and the one 2800 gap clears 8/5 of it, so
                             the leading-gap half of the block cut opens exactly two blocks of
                             three lines. The first fixture AUTHORED for that cut: three earlier
                             ones (`both-table-rules`, `rotated-and-mirrored-text`,
                             `stroke-ruled-worksheet`) come out in two blocks by accident of a
                             layout built for something else, and none states its leading or
                             its gap, so none can say which gap opened a block or that the
                             threshold was cleared on purpose                  [OPEN-WORK §2.2]

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
# through crates/ethos-parser-pdf/src/metrics.rs, and the difference between the last two is the whole
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

# The face a "no-metrics" fixture must name, and it deliberately is NOT one of the standard 14.
#
# Decision #22 vendored Adobe's Core-14 AFMs, so `/BaseFont /Helvetica` now HAS a knowable ascent
# and descent whatever the descriptor says — §9.6.2.2 makes those metrics known and merely absent
# from the file. A Helvetica fixture can therefore no longer demonstrate typed-absent geometry:
# it was silently exercising the recovered path instead, which is a test asserting nothing.
#
# `ArialMT` is the case decision #22 explicitly refuses to fill: supplying Helvetica's metrics for
# Arial is a metric SUBSTITUTION rather than a reading. So this face has no metrics anywhere —
# not in the document, and not in `vendor/afm/` — which is the only remaining way to reach the
# geometry-omission path honestly.
UNKNOWABLE_FACE = "ArialMT"


def two_column_stream(left_lines: int, right_lines: int) -> str:
    """A two-column page, written RIGHT column first, on a 400x300 media box.

    v1-S5's anti-cliff pair. pdf-inspector decides multi-column on `min_lines < 15`: fourteen
    lines on a page come out row-interleaved and fifteen come out column-major, so a one-line
    edit reorders the whole document (docs/history/03-V0-SCOPE.md 3.2). Seven lines per column is
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


def _form_object() -> bytes:
    """Object 6 for the form-XObject fixture: a /Form that draws text (v2.2-S2).

    Deliberately WELL-FORMED — /BBox, /Resources naming object 5, an uncompressed stream a
    reviewer can read. A malformed form would also produce zero nodes, and then the fixture would
    prove that a broken stream is skipped rather than that a working one is not descended into.
    """
    inner = b"BT /F1 12 Tf 1 0 0 1 0 6 Tm (Drawn inside the form) Tj ET"
    return (
        b"<< /Type /XObject /Subtype /Form /FormType 1 /BBox [0 0 220 24] "
        b"/Resources << /Font << /F1 5 0 R >> >> /Length %d >>\nstream\n%s\nendstream"
        % (len(inner), inner)
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
    font_object=None,
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
    # `font_object` REPLACES object 5 outright, and exists because a composite font is not the
    # simple shape with extra keys — it is a different dictionary (v2.2-S3). A /Type0 carries no
    # /FirstChar, no /LastChar and no /Widths at all; its widths live on a descendant CIDFont and
    # are keyed by CID. Splicing that through `font_extra` would have produced a font dictionary
    # holding BOTH shapes, and a fixture that is legal-but-nonsense proves nothing about a reader
    # that has to choose between them. When it is given, `font_subtype`, `differences` and
    # `descriptor` have nothing to act on and are refused rather than silently ignored.
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
    assert not (font_object and (descriptor or differences or font_extra)), (
        "font_object replaces the whole font dictionary, so a descriptor, /Differences or "
        "font_extra would be silently dropped; give the fixture one or the other"
    )
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
        font_object
        if font_object
        else (
            "<< /Type /Font /Subtype /%s /BaseFont /%s "
            "/Encoding %s /FirstChar %d /LastChar %d /Widths [%s]%s%s >>"
            % (
                font_subtype,
                UNKNOWABLE_FACE if descriptor == "no-metrics" else "Helvetica",
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
            b"<< /Type /FontDescriptor /FontName /%s /Flags 32 "
            b"/ItalicAngle 0 /StemV 88 >>" % UNKNOWABLE_FACE.encode()
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
    # v1.1-S1's MARKDOWN GOLDEN. Two runs, both in a font declaring real ink metrics, so both
    # reach `ethos.grounding.v1` as elements a verifier can actually find.
    #
    # That combination is why this fixture had to exist. The projection turns two runs into
    # `First block\n\nSecond block`, and the blank line between them is a `syntax` segment — bytes
    # this exporter invented. `block\n\nSecond` is then real text in the Markdown that the page
    # never drew, which is exactly the string the Anchor Map exists to mark unquotable. Proving
    # that needs the verifier to ground the first half and refuse the second, and every existing
    # fixture with measurable ink has only ONE run, so there is no join to span.
    # v2.2-S5's UNDECLARED-JOIN fixture. Every other engine fixture puts its runs on distinct
    # baselines, so the whole CLI suite was blind to the fallback: a rule keyed on "same baseline,
    # next ink along it" changed nothing anywhere and passed. This is the tripwire.
    #
    # Uniform /Widths of 500 at 24pt is 12 points per glyph, so the abutment is arithmetic a
    # reviewer can check without running anything:
    #
    #     72 + 3x12 = 108     `Yar` ends where `ro` starts
    #    108 + 2x12 = 132     `ro`  ends where `w`  starts
    #    132 + 1x12 = 144     `w`   ends, and the next run starts 40 points further on
    #
    # So `Yarrow` is one block and `Separate` is another, and a rule that joined on absence alone
    # would produce `YarrowSeparate` across a gap the page plainly drew.
    "untagged-shredded-line": (
        "BT /F1 24 Tf 72 100 Td (Yar) Tj ET "
        "BT /F1 24 Tf 108 100 Td (ro) Tj ET "
        "BT /F1 24 Tf 132 100 Td (w) Tj ET "
        "BT /F1 24 Tf 184 100 Td (Separate) Tj ET"
    ),
    "markdown-two-blocks": (
        "BT /F1 24 Tf 72 120 Td (First block) Tj ET "
        "BT /F1 24 Tf 72 60 Td (Second block) Tj ET"
    ),
    # v1.1-S3's HYPHEN GOLDEN, and a third fixture for the same reason the second one existed.
    #
    # `synthetic/hyphenated-line-break` in the Ethos corpus already carries the shape — two runs,
    # `hyphen-` then `ated` — but its font declares NO ink metrics, so both runs take the
    # typed-absent path, `ethos.grounding.v1` comes out with an empty `elements` array, and a
    # golden against it would watch the verifier find nothing and refuse every quote. That proves
    # nothing about the join.
    #
    # Two lines at 12pt, 30 points apart, in the metrics font:
    #
    #     y120  The rate may be recalcu-
    #      y90  lated at closing
    #
    # The projection closes that up to `The rate may be recalculated at closing`, and THAT is the
    # sentence the golden is about. It reads perfectly. A model handed the Markdown would cite it
    # without hesitation. It is on no page: the document drew `recalcu-` and `lated`, and no
    # element of the grounding artifact contains the joined word. Both halves ground; the joined
    # sentence comes back `text_mismatch` against an element that DOES exist, which is the whole
    # cost of the cosmetic made executable.
    #
    # 24 characters at 12pt and a uniform 500/1000 width is 144 points, so from x=40 the longer
    # line ends at 184 on a 300-wide page — inside it, deliberately, because a run that fell off
    # the page would take the off-page path and the fixture would be proving that instead.
    "markdown-hyphen-break": (
        "BT /F1 12 Tf 40 120 Td (The rate may be recalcu-) Tj ET "
        "BT /F1 12 Tf 40 90 Td (lated at closing) Tj ET"
    ),
    # v1.1-S2's GFM GOLDEN, and the reason it is a second fixture rather than a reused one.
    # `ruled-table-grid` already has a merge and an empty cell, but its font declares NO ink
    # metrics, so every run's geometry is typed-absent and none of them reaches
    # `ethos.grounding.v1`. A cell-quote golden against that fixture would watch the verifier find
    # nothing and refuse both halves, which proves nothing about cells.
    #
    # A 3x3 lattice, stroked as `re` cell rectangles, under a font that declares real metrics:
    #
    #     x:  40      140     240     340
    #     y160 +-------+-------+-------+
    #          | Region| A|B   | Total |   row 0
    #     y120 +-------+---------------+
    #          | North |  merged span  |   row 1, cols 1-2 MERGED
    #      y80 +-------+-------+-------+
    #          |       |       |       |   row 2, wholly EMPTY
    #      y40 +-------+-------+-------+
    #
    # Three properties are deliberate, and each is an S2 acceptance test.
    #
    # The **merge** is the erasure GFM cannot represent: the text goes in the origin slot and the
    # covered slot comes out empty, counted as `gfm-span-slots-unrepresentable-v1`.
    #
    # The **pipe** inside `A|B` is the character GFM reads as a cell boundary. The projection
    # escapes it — and the backslash is `syntax` while the pipe stays `source`, so a quote
    # containing the pipe still inverts to the run that drew it.
    #
    # The **empty last row** is the competitor erasure checklist A14 names: a serializer that
    # truncates trailing empties makes a prettier table and a different document. It stays.
    "markdown-table-cells": (
        "1 w "
        # row 0
        "40 120 100 40 re S 140 120 100 40 re S 240 120 100 40 re S "
        # row 1: col 0, then one rectangle spanning columns 1 and 2
        "40 80 100 40 re S 140 80 200 40 re S "
        # row 2, drawn and never written in
        "40 40 100 40 re S 140 40 100 40 re S 240 40 100 40 re S "
        "BT /F1 12 Tf "
        "1 0 0 1 50 134 Tm (Region) Tj 1 0 0 1 150 134 Tm (A|B) Tj "
        "1 0 0 1 250 134 Tm (Total) Tj "
        "1 0 0 1 50 94 Tm (North) Tj 1 0 0 1 150 94 Tm (merged span) Tj "
        "ET"
    ),
    # v1.1-S2's LIST fixture, and it had to be authored because **neither corpus tags a list**.
    # A projection that grew list markers from bullet glyphs or hanging indents would be reading
    # layout, which is the same refusal L29 makes about font-size headings one level up — so the
    # only way to prove the branch is a document whose structure tree actually says `/L`.
    #
    # Every run sits on its own baseline rather than sharing one with its label. A real list draws
    # `1.` and `First item` on one line; putting them on two changes nothing this fixture tests —
    # the projection reads the TREE, never a coordinate — and it keeps the reading-order rule out
    # of a test that is not about reading order.
    #
    # Four things are proved here and nowhere else:
    #   /Lbl 0 + /LBody 1              an item whose own marker text the document drew
    #   a nested /L under /LBody       depth, from nested `/L` rather than from indentation
    #   /LBody with /K [7 8]           TWO runs in one item, which is the join S2 counts
    #   an unmarked closing run        a non-list run ends the list rather than joining it
    "tagged-list-items": (
        "BT /F1 12 Tf "
        "/Lbl <</MCID 0>> BDC 1 0 0 1 40 180 Tm (1.) Tj EMC "
        "/LBody <</MCID 1>> BDC 1 0 0 1 60 164 Tm (First item) Tj EMC "
        "/Lbl <</MCID 2>> BDC 1 0 0 1 40 148 Tm (2.) Tj EMC "
        "/LBody <</MCID 3>> BDC 1 0 0 1 60 132 Tm (Second item) Tj EMC "
        "/Lbl <</MCID 4>> BDC 1 0 0 1 60 116 Tm (a.) Tj EMC "
        "/LBody <</MCID 5>> BDC 1 0 0 1 80 100 Tm (Nested item) Tj EMC "
        "/Lbl <</MCID 6>> BDC 1 0 0 1 40 84 Tm (3.) Tj EMC "
        "/LBody <</MCID 7>> BDC 1 0 0 1 60 68 Tm (Third item) Tj EMC "
        "/LBody <</MCID 8>> BDC 1 0 0 1 60 52 Tm (continued) Tj EMC "
        "1 0 0 1 40 30 Tm (Closing paragraph) Tj "
        "ET"
    ),
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
    # Decision #22's other side. `absent-font-metrics` supplies /Widths and withholds ink; this
    # one withholds BOTH, on a face no vendored table can answer for, which is the only shape
    # left that reaches `font-widths-absent` and an absent advance at once.
    "absent-font-widths": "BT /F1 12 Tf 1 0 0 1 72 72 Tm (No widths anywhere) Tj ET",
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
    # v1-S8's STROKE-RULED golden, and the shape of `cfpb-home-loan-toolkit` page 13.
    #
    # Five baselines and four column spans, drawn as two-point `m`/`l` pairs stroked with `S` —
    # the ink `content::flush_subpath` produced nothing at all from before this slice, because it
    # wants four or five points before it will call a subpath a rectangle.
    #
    # The THIRD baseline from the top rules three cells instead of four. That is the blank first
    # cell a worksheet leaves for its reader, and it is what `stroke-ruled-v1` refused the whole
    # band over. The three INTERIOR column rules are stroked and the two outer ones are not, which
    # is also what page 13 does — an outer edge is where the ink stops.
    #
    # Five baselines bound FOUR rows, and the fixture's top row of text sits above the first
    # baseline precisely so the missing top edge is visible: the page shows five rows and the
    # engine emits four, which is `undrawn-table-edges-not-supplied` in one picture.
    "stroke-ruled-worksheet": (
        "0.5 w "
        # Each baseline is FOUR abutting segments, as page 13 draws its own: a rule-row has to
        # tile end to end before it is a row of cells, so one long rule is one long rule.
        "20 160 m 80 160 l S 80 160 m 140 160 l S "
        "140 160 m 200 160 l S 200 160 m 260 160 l S "
        "20 130 m 80 130 l S 80 130 m 140 130 l S "
        "140 130 m 200 130 l S 200 130 m 260 130 l S "
        # THREE segments: this row's first cell is blank and its rule is simply not drawn.
        "80 100 m 140 100 l S 140 100 m 200 100 l S 200 100 m 260 100 l S "
        "20 70 m 80 70 l S 80 70 m 140 70 l S "
        "140 70 m 200 70 l S 200 70 m 260 70 l S "
        "20 40 m 80 40 l S 80 40 m 140 40 l S "
        "140 40 m 200 40 l S 200 40 m 260 40 l S "
        # The three INTERIOR column rules. The outer two at x=20 and x=260 are NOT drawn.
        "80 160 m 80 40 l S "
        "140 160 m 140 40 l S "
        "200 160 m 200 40 l S "
        "BT /F1 9 Tf "
        "1 0 0 1 24 168 Tm (Item) Tj 1 0 0 1 84 168 Tm (One) Tj "
        "1 0 0 1 144 168 Tm (Two) Tj 1 0 0 1 204 168 Tm (Three) Tj "
        "1 0 0 1 24 138 Tm (Lender) Tj "
        "1 0 0 1 24 78 Tm (Rate) Tj "
        "1 0 0 1 24 48 Tm (Term) Tj "
        "ET"
    ),
    # v1-S8's OVER-DETECTION negative: the same worksheet with its column rules NOT drawn.
    #
    # Byte for byte the horizontal ink of `stroke-ruled-worksheet` and none of the vertical. The
    # rules still end at a common x, so the band and its column lines are built exactly as before
    # — and refused, because nothing on the page says the author divided there. This is the shape
    # of `cfpb-home-loan-toolkit`'s Closing Disclosure pages, which `stroke-ruled-v1` turned into
    # six tables on pages nobody tagged.
    "stroke-ruled-columns-not-drawn": (
        "0.5 w "
        "20 160 m 80 160 l S 80 160 m 140 160 l S "
        "140 160 m 200 160 l S 200 160 m 260 160 l S "
        "20 130 m 80 130 l S 80 130 m 140 130 l S "
        "140 130 m 200 130 l S 200 130 m 260 130 l S "
        "80 100 m 140 100 l S 140 100 m 200 100 l S 200 100 m 260 100 l S "
        "20 70 m 80 70 l S 80 70 m 140 70 l S "
        "140 70 m 200 70 l S 200 70 m 260 70 l S "
        "20 40 m 80 40 l S 80 40 m 140 40 l S "
        "140 40 m 200 40 l S 200 40 m 260 40 l S "
        "BT /F1 9 Tf "
        "1 0 0 1 24 138 Tm (Lender) Tj "
        "1 0 0 1 24 78 Tm (Rate) Tj "
        "ET"
    ),
    # v1-S8's FORM negative, and the `irs-form-1040-2025` principle without the tax form.
    #
    # A 2x2 grid whose four faces are drawn as ruling lines AND declared as four widget /Rects at
    # the same coordinates. The boxes are the fields' own frames, so this is a form rather than a
    # table, and every slice since v1-S1 has held that document at zero tables.
    #
    # Contrast `stroke-ruled-worksheet`, which carries no widgets at all, and the real page 13,
    # which carries 25 of them INSET inside its printed cells and is still a table.
    "stroke-ruled-field-boxes": (
        "0.5 w "
        "20 120 m 140 120 l S 140 120 m 260 120 l S "
        "20 90 m 140 90 l S 140 90 m 260 90 l S "
        "20 60 m 140 60 l S 140 60 m 260 60 l S "
        "140 120 m 140 60 l S "
        "BT /F1 9 Tf 1 0 0 1 24 128 Tm (Amounts) Tj ET"
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
    # v2.2-S3. Four glyphs under Identity-H, written as 2-byte codes because that is what an
    # Identity CMap means. Their four widths come from four different places — /W array form, /W
    # array form, the font's own /DW, /W range form — and are 500, 750, 900 and 250 glyph units.
    # At 12 pt that is 6.0 + 9.0 + 10.8 + 3.0 = 28.8 pt of advance, and no two of the four are
    # equal, so a reader that mis-sourced any single one lands on a different total.
    "composite-font-cid-widths": (
        "BT /F1 12 Tf 1 0 0 1 40 100 Tm <0001000200030005> Tj ET"
    ),
    # The same four glyphs under a CMap this profile does not parse. The text still decodes —
    # /ToUnicode is authoritative for characters — and the ADVANCE must be absent, because the
    # code is not the CID and nothing here can say what is.
    "composite-font-non-identity-cmap": (
        "BT /F1 12 Tf 1 0 0 1 40 100 Tm <0001000200030005> Tj ET"
    ),
    # The SAME image, declared and never drawn. `Do` is what makes a node; a resource nobody
    # painted is a resource, and zero image nodes is the correct answer.
    "image-declared-not-drawn": "BT /F1 12 Tf 1 0 0 1 40 60 Tm (No Do here) Tj ET",
    # v2.2-S2's golden. The same `Do` as `image-xobject-drawn`, on an XObject whose /Subtype is
    # /Form rather than /Image. One sentence is drawn by the PAGE and one by the FORM, so the
    # fixture holds both halves of the claim in one file: the page's sentence must be a node, the
    # form's must not be, and the count is the only thing on the artifact that can tell a consumer
    # the second one exists at all.
    #
    # The `q`/`Q` bracket and the `cm` are the image fixture's, unchanged. Nothing about the
    # placement is the subject here — the /Subtype is — and keeping the surrounding operators
    # identical is what makes the difference between the two fixtures readable by eye.
    "form-xobject-text-drawn": (
        "BT /F1 12 Tf 1 0 0 1 40 100 Tm (Drawn by the page) Tj ET "
        "q 1 0 0 1 40 40 cm /Xf1 Do Q"
    ),
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
    # D4-S5's golden: INK, measured correctly, drawn outside the MEDIA box by the document.
    #
    # The three fixtures above it each stop one step short of this case. `crop-box-smaller-than-
    # media` puts a measured box outside the CROP box and inside the media box. `off-page-and-
    # offset-box` puts a run's origin outside the crop box. `whitespace-past-the-page-edge` puts a
    # box outside the media box but around NOTHING, which v1-S6.2 answered by not claiming a box.
    # None of them has ink outside the media box, and that is the case the corpus met in the wild:
    # six of two hundred DP-Bench documents, each a page lifted from a wider original, each
    # producing no artifact at all.
    #
    # `1 0 0 1 -260 100 Tm` against the 300x144 default media box puts the whole run at negative x,
    # the way `01030000000029.pdf` does at -435.1181. The second run is inside the page and is the
    # other half of the assertion: a document that draws off-canvas still gets real boxes for the
    # content that is on the canvas, so the absence is per-run and never a page-wide give-up.
    "ink-past-the-media-box": (
        "BT /F1 12 Tf 1 0 0 1 -260 100 Tm (Off the left edge) Tj "
        "1 0 0 1 40 60 Tm (On the page) Tj ET"
    ),
    # docs/22 §9 items 1 and 2's golden: one run per way a baseline can be turned.
    #
    # The text matrix turns five of them — a quarter turn up the page, a quarter turn down it,
    # upside down, mirrored, and 45 degrees — and the CTM turns the seventh. Through 0.57.0 the
    # advance was read from `Tm.e` alone and scaled by the CTM's length, so the first four advanced
    # 0 or less and were typed as drawing nothing, the 45-degree run got a foreshortened upright
    # box, and the CTM-turned run got a box along x. `Upright` is the control: its box must not
    # move. `Diagonal` has no axis-aligned rectangle at all and must say so.
    "rotated-and-mirrored-text": (
        "BT /F1 12 Tf 1 0 0 1 20 20 Tm (Upright) Tj "
        "0 1 -1 0 40 60 Tm (Turned) Tj "
        "0 -1 1 0 80 240 Tm (Downward) Tj "
        "-1 0 0 -1 250 270 Tm (Inverted) Tj "
        "-1 0 0 1 280 200 Tm (Mirrored) Tj "
        "0.7071 0.7071 -0.7071 0.7071 150 120 Tm (Diagonal) Tj ET "
        "q 0 1 -1 0 300 0 cm BT /F1 12 Tf 1 0 0 1 100 180 Tm (Rolled) Tj ET Q"
    ),
    # The block cut's own fixture (OPEN-WORK §2.2): six single-run lines in one column, three at
    # a 14 pt leading, then a 28 pt gap, then three more at 14 pt. In the rule's own units —
    # centipoints, top-left origin, on the 720 pt page MEDIA gives it — the baselines are 2000,
    # 3400, 4800, 7600, 9000 and 10400:
    #
    #     gaps            1400 1400 2800 1400 1400
    #     modal leading   1400   (four of five gaps: share 4/5 >= 1/4, and 1400 >= 600)
    #     threshold       8/5 x 1400 = 2240
    #
    # The 2800 gap clears the threshold and the 1400 gaps do not, so exactly one cut falls
    # between the third line and the fourth: lines 1-3 are block 1 and lines 4-6 are block 2,
    # and every run's block can be checked against `blocks.rs` by hand. Plain words, one `Tj` per
    # line, written top to bottom so stream order and reading order agree and nothing but the gap
    # is being tested. No earlier fixture states its leading or its gap: three come out in two
    # blocks by accident (see the header), and the two-run ones cannot, because one gap is its
    # own leading and a gap never clears 1.6 times itself.
    "leading-gap-two-blocks": (
        "BT /F1 12 Tf "
        "1 0 0 1 72 700 Tm (Water finds its level) Tj "
        "1 0 0 1 72 686 Tm (and stone keeps its shape) Tj "
        "1 0 0 1 72 672 Tm (through the long season) Tj "
        "1 0 0 1 72 644 Tm (Wind moves the grass) Tj "
        "1 0 0 1 72 630 Tm (and light moves the shade) Tj "
        "1 0 0 1 72 616 Tm (across the open field) Tj "
        "ET"
    ),
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
    # v1.1-S2's tagged list. `/L` holds three `/LI`; each `/LI` holds its own `/Lbl` and `/LBody`,
    # which is how PDF 32000-1 s14.8.4.3 spells a list and the only shape this projection reads.
    #
    # Object 14's `/K` is MIXED — `[3 15 0 R]` — a marked-content id AND a child element, which is
    # how a real nested list is written: the second item's body is both its own text and the
    # sub-list under it. Object 21's `/K` is `[7 8]`, two marked-content ids in ONE `/LBody`,
    # which is the case where two sibling `/LI`s and one wrapped item are indistinguishable by
    # role path alone — the join `gfm-list-item-run-joins-v1` counts.
    "tagged-list-items": [
        "<< /Type /StructTreeRoot /K 7 0 R >>",
        "<< /Type /StructElem /S /Document /P 6 0 R /K [8 0 R] >>",
        "<< /Type /StructElem /S /L /P 7 0 R /K [9 0 R 12 0 R 19 0 R] >>",
        "<< /Type /StructElem /S /LI /P 8 0 R /K [10 0 R 11 0 R] >>",
        "<< /Type /StructElem /S /Lbl /P 9 0 R /Pg 3 0 R /K 0 >>",
        "<< /Type /StructElem /S /LBody /P 9 0 R /Pg 3 0 R /K 1 >>",
        "<< /Type /StructElem /S /LI /P 8 0 R /K [13 0 R 14 0 R] >>",
        "<< /Type /StructElem /S /Lbl /P 12 0 R /Pg 3 0 R /K 2 >>",
        "<< /Type /StructElem /S /LBody /P 12 0 R /Pg 3 0 R /K [3 15 0 R] >>",
        "<< /Type /StructElem /S /L /P 14 0 R /K [16 0 R] >>",
        "<< /Type /StructElem /S /LI /P 15 0 R /K [17 0 R 18 0 R] >>",
        "<< /Type /StructElem /S /Lbl /P 16 0 R /Pg 3 0 R /K 4 >>",
        "<< /Type /StructElem /S /LBody /P 16 0 R /Pg 3 0 R /K 5 >>",
        "<< /Type /StructElem /S /LI /P 8 0 R /K [20 0 R 21 0 R] >>",
        "<< /Type /StructElem /S /Lbl /P 19 0 R /Pg 3 0 R /K 6 >>",
        "<< /Type /StructElem /S /LBody /P 19 0 R /Pg 3 0 R /K [7 8] >>",
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



# --- v2.2-S3: the composite font, the shape NEITHER owned corpus contained ---------------------
#
# Before this slice `grep -l CIDFontType fixtures/` matched nothing, in either corpus. The whole
# composite-width path was therefore exercised by no test at all, which is why `load_widths`
# reading `/Widths` off a /Type0 dictionary — a key the format never puts there — passed 1 294
# tests for its entire life. Same argument v1-S6 used to justify `image-xobject-drawn`: *"the
# corpus contains NO image XObject anywhere, so without this the whole image path is untested."*

# Codes are CIDs under Identity-H, so the CMap maps the four CIDs the page draws.
_CID_TOUNICODE = (
    b"/CIDInit /ProcSet findresource begin\n"
    b"12 dict begin\nbegincmap\n"
    b"1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n"
    b"4 beginbfchar\n"
    b"<0001> <0041>\n"  # A
    b"<0002> <0042>\n"  # B
    b"<0003> <0043>\n"  # C
    b"<0005> <0045>\n"  # E
    b"endbfchar\n"
    b"endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend"
)


def _type0_font(encoding: str) -> bytes:
    """Object 5: a /Type0 font. Note what it does NOT carry: /FirstChar, /LastChar, /Widths."""
    return (
        "<< /Type /Font /Subtype /Type0 /BaseFont /Helvetica /Encoding /%s "
        "/DescendantFonts [6 0 R] /ToUnicode 7 0 R >>" % encoding
    ).encode()


def _cid_font() -> bytes:
    """Object 6: the descendant CIDFont, and the whole point of the pair of fixtures.

    `/W` deliberately uses BOTH forms PDF 32000-1 sec 9.7.4.3 defines, because both occur in the
    wild — 1 143 array-form and 810 range-form entries across the 200-document
    `opendataloader-bench` corpus — and a parser that implemented one would read the other's
    numbers as CIDs and produce silently wrong spans:

        1 [500 750]   the ARRAY form: CID 1 -> 500, CID 2 -> 750
        5 7 250       the RANGE form: CIDs 5, 6 and 7 -> 250

    CID 3 is named by neither and falls to `/DW`. So the four glyphs the page draws take their
    widths from four different code paths, and the four numbers are all different — a reader that
    got any single one wrong produces a different total advance.

    **`/DW` is 900 and not 1000, and that is the whole reason this fixture can see it.** 1000 is
    also the value sec 9.7.4.3 gives when `/DW` is omitted, so a `/DW 1000` here would be
    indistinguishable from the default and a reader that ignored the key entirely would still
    produce the right number. Measured, not argued: with `/DW 1000` a mutant replacing the default
    with zero SURVIVED this fixture. The absent-`/DW` case is covered by a unit test in
    `fonts.rs`, where a dictionary can be built without one.
    """
    return (
        b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Helvetica "
        b"/CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> "
        b"/DW 900 /W [ 1 [500 750] 5 7 250 ] /FontDescriptor 8 0 R >>"
    )


def _cid_tounicode() -> bytes:
    return b"<< /Length %d >>\nstream\n%s\nendstream" % (
        len(_CID_TOUNICODE),
        _CID_TOUNICODE,
    )


def _cid_descriptor() -> bytes:
    """Object 8: REAL ascent/descent, so an ink box is measured once a width exists.

    Without this the fixture would prove only that the advance was read. The advance is what the
    ink box needs to become a rectangle, and the rectangle is what makes a node groundable — which
    is the thing the defect actually cost. A descriptor with no metrics would leave the node
    ungroundable for an unrelated reason and the fixture would assert nothing about the repair.
    """
    return (
        b"<< /Type /FontDescriptor /FontName /Helvetica /Flags 4 "
        b"/Ascent 718 /Descent -207 /ItalicAngle 0 /StemV 88 "
        b"/FontBBox [-166 -225 1000 931] >>"
    )


# A SIMPLE font dictionary written out in full, for a fixture whose point is something
# `build_pdf` always supplies. It always writes `/Widths`, so a document without them cannot be
# expressed any other way.
#
# `/ArialMT` is deliberate and load-bearing since decision #22: Helvetica's widths and ink metrics
# are now readable from `vendor/afm/` whether or not the document carries them, so a Helvetica
# fixture can no longer reach the width-absent path at all. Arial is the metric SUBSTITUTION that
# decision refuses, so nothing can answer for it — which is what keeps this fixture honest.
RAW_FONTS = {
    "absent-font-widths": (
        b"<< /Type /Font /Subtype /Type1 /BaseFont /ArialMT /Encoding /WinAnsiEncoding >>"
    ),
}

COMPOSITE_FONTS = {
    "composite-font-cid-widths": _type0_font("Identity-H"),
    # The SAME descendant, the same /W, the same /DW — and a predefined CMap this profile does not
    # parse. `/W` is keyed by CID and the code -> CID map is that CMap, so the CID is unknown and
    # a width read here would be a plausible number for the wrong glyph. Must refuse, and must
    # refuse for THAT reason: the standard-14 AFM sentence is about a different case entirely and
    # was the wrong explanation printed 50 times out of 51 before this slice.
    "composite-font-non-identity-cmap": _type0_font("UniJIS-UCS2-H"),
}

COMPOSITE_OBJECTS = {
    name: [_cid_font(), _cid_tounicode(), _cid_descriptor()] for name in COMPOSITE_FONTS
}


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
    # v2.2-S2. Object 6 again, and a /Form this time. Its /Resources names the page's own font
    # object rather than a copy, so the text inside it is drawable by any reader that descends —
    # this one does not, and the fixture is worth nothing if the reason is "the form was broken"
    # instead of "the profile does not descend".
    "form-xobject-text-drawn": [_form_object()],
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
    # v1-S8. Four widgets whose /Rects ARE the four faces of the stroked 2x2 above, so the grid
    # is the form's field boxes rather than a table's cells. In user space, as /Rect always is.
    "stroke-ruled-field-boxes": [
        "<< /Fields [7 0 R 8 0 R 9 0 R 10 0 R] >>",
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T (a1) /Rect [20 90 140 120] /P 3 0 R >>",
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T (a2) /Rect [140 90 260 120] /P 3 0 R >>",
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T (b1) /Rect [20 60 140 90] /P 3 0 R >>",
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T (b2) /Rect [140 60 260 90] /P 3 0 R >>",
    ],
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
    "stroke-ruled-field-boxes": " /AcroForm 6 0 R",
    "form-orphan-widget": " /AcroForm 6 0 R",
    "form-xfa-stub": " /AcroForm 6 0 R",
}

# name -> raw fragment spliced into the page dictionary.
PAGE_EXTRA = {
    "form-field-value": " /Annots [7 0 R]",
    "stroke-ruled-field-boxes": " /Annots [7 0 R 8 0 R 9 0 R 10 0 R]",
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
    # v1.1-S2. Same lattice geometry as ruled-table-grid, and the same page to hold it.
    "markdown-table-cells": (0, 0, 400, 200),
    # v1.1-S2. Ten baselines at 16pt spacing, from y=180 down to y=30.
    "tagged-list-items": (0, 0, 300, 200),
    # Tall enough for two 24pt lines with real ink boxes inside the page.
    "untagged-shredded-line": (0, 0, 300, 200),
    "markdown-two-blocks": (0, 0, 300, 200),
    # v1.1-S3. Two 12pt lines 30 points apart, and wide enough that the longer one ends at 184.
    "markdown-hyphen-break": (0, 0, 300, 200),
    "ruled-table-overlap": (0, 0, 300, 160),
    "unruled-near-miss": (0, 0, 300, 200),
    "background-panel-not-a-grid": (0, 0, 240, 200),
    # v1-S8. Wide enough for four 60pt columns plus margins, tall enough for five baselines
    # and a row of headings above the topmost one.
    "stroke-ruled-worksheet": (0, 0, 280, 200),
    "stroke-ruled-columns-not-drawn": (0, 0, 280, 200),
    "stroke-ruled-field-boxes": (0, 0, 280, 160),
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
    # Square, so a run turned a quarter has as much room as an upright one.
    "rotated-and-mirrored-text": (0, 0, 300, 300),
    # Tall enough that a baseline at 700 keeps its measured ink on the page (ascent 718 at 12 pt
    # is 8.6 pt), and no wider than it needs to be: the longest line is 25 glyphs at 6 pt.
    "leading-gap-two-blocks": (0, 0, 300, 720),
}

# name -> /Resources fragment. Only the image fixtures declare an /XObject.
RESOURCES_EXTRA = {
    "image-xobject-drawn": " /XObject << /Im1 6 0 R >>",
    "image-declared-not-drawn": " /XObject << /Im1 6 0 R >>",
    "form-xobject-text-drawn": " /XObject << /Xf1 6 0 R >>",
}

# name -> /Differences array body. Only the broken-encoding fixture carries one.
DIFFERENCES = {
    "broken-font-encoding": "200 /nonexistentglyphone /nonexistentglyphtwo /nonexistentglyphthree",
}

# name -> descriptor kind. Absent from this map means no descriptor at all.
DESCRIPTORS = {
    "measured-ink-box": "metrics",
    # Real metrics on BOTH runs, so both ground. Without them the elements array is empty and the
    # golden would pass vacuously against a verifier that found nothing either way.
    "untagged-shredded-line": "metrics",
    "markdown-two-blocks": "metrics",
    # v1.1-S2. Real metrics so the CELL runs are groundable elements; without them the cell-quote
    # golden would watch the verifier find nothing and refuse both halves, proving nothing about
    # cells. This is also why the list fixture has no descriptor: a /FontDescriptor and a
    # structure tree both claim object 6, and build_pdf refuses a fixture that wants both.
    "markdown-table-cells": "metrics",
    # v1.1-S3. Real metrics so BOTH halves of the broken word reach `ethos.grounding.v1`. Without
    # them the elements array is empty, the verifier finds nothing, and the golden would refuse
    # the joined sentence for a reason that has nothing to do with the join.
    "markdown-hyphen-break": "metrics",
    "absent-font-metrics": "no-metrics",
    # Real metrics, so the ink box is MEASURED — without this the fixture proves nothing, because
    # a typed-absent box can never fall outside a page.
    "crop-box-smaller-than-media": "metrics",
    # Real metrics, so an ink box would be built. Without them the run takes the typed-absence
    # path for a different reason and the fixture proves nothing.
    "whitespace-past-the-page-edge": "metrics",
    # D4-S5. Real metrics for the same reason `crop-box-smaller-than-media` needs them: a
    # typed-absent box can never be found outside a page, so without these the fixture would
    # assert nothing about the case it exists for.
    "ink-past-the-media-box": "metrics",
    # Real metrics, so every turned run has a box to build — or a typed reason it has none.
    "rotated-and-mirrored-text": "metrics",
    # Real metrics so all six runs reach the grounding artifact, as `measured-ink-box` does. The
    # cut reads origins only, so the metrics change nothing about which block a run lands in.
    "leading-gap-two-blocks": "metrics",
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
                or COMPOSITE_OBJECTS.get(name)
            ),
            font_object=COMPOSITE_FONTS.get(name) or RAW_FONTS.get(name),
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
