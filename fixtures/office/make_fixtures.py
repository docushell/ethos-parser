#!/usr/bin/env python3
# Copyright 2026 The ethos-parser maintainers
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
"""Author the v2 office fixtures, deterministically.

The counterpart to `fixtures/engine/make_fixtures.py`, and authored here for the same reason: a
fixture nobody can regenerate is a fixture nobody can reason about. These are **engine-owned**
documents — not a sample from a vendor, not a file with a licence somebody has to check.

Determinism matters as much as content. `zipfile` stamps a timestamp into every entry by default,
which would make two runs produce two different `source.sha256` values and turn the double-run
byte-identity test into a coin flip. Every entry below is written with a fixed date.

Run:  python3 fixtures/office/make_fixtures.py
"""

import pathlib
import zipfile

HERE = pathlib.Path(__file__).parent
FIXED_DATE = (2026, 1, 1, 0, 0, 0)

# A 1x1 PNG, authored here rather than sampled. Every fixture that needs an embedded asset uses
# this one: the point is never the image, it is that a package entry holding something no reader
# reads is COUNTED, and the smallest valid file makes that point without adding a licence to check.
PICTURE = bytes.fromhex(
    "89504e470d0a1a0a0000000d494844520000000100000001080600000"
    "01f15c4890000000a49444154789c6300010000050001"
    "0d0a2db40000000049454e44ae426082"
)

CONTENT_TYPES = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>
"""

RELS = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>
"""

# Three paragraphs, five runs. The second paragraph splits a sentence across two runs with
# `xml:space="preserve"` on the second, which is what makes the space-preserved flag testable
# rather than always false. The ampersand is there so entity decoding is exercised by a real
# fixture and not only by a unit test.
DOCUMENT = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>Evidence, not extraction.</w:t></w:r></w:p>
    <w:p>
      <w:r><w:t>A quote binds to a run</w:t></w:r>
      <w:r><w:t xml:space="preserve"> and never to a page.</w:t></w:r>
    </w:p>
    <w:p>
      <w:r><w:br/></w:r>
      <w:r><w:t>Rows &amp; columns are S3.</w:t></w:r>
    </w:p>
  </w:body>
</w:document>
"""

PARTS = {
    "[Content_Types].xml": CONTENT_TYPES,
    "_rels/.rels": RELS,
    "word/document.xml": DOCUMENT,
}


# v2-S11. A package that stores a `.png` must declare the extension's content type; a package
# without one must not gain the declaration, or every clean fixture's `source.sha256` moves and
# takes a golden with it. So the media fixtures get a DERIVED content-types part rather than a
# widened shared one.
def with_png(content_types: str) -> str:
    return content_types.replace(
        '<Default Extension="xml" ContentType="application/xml"/>',
        '<Default Extension="xml" ContentType="application/xml"/>\n'
        '  <Default Extension="png" ContentType="image/png"/>',
        1,
    )


def write(path: pathlib.Path, parts: dict, stored_first=None) -> None:
    """Write a deterministic package.

    `stored_first` names an entry that must be written FIRST and UNCOMPRESSED. Only ODF needs it,
    and it is not a convenience: the OpenDocument package specification requires the `mimetype`
    entry to be the first file and to be stored, so that a consumer can identify the document from
    the leading bytes without inflating anything. `ethos-parser-office`'s reader checks both, so a
    fixture that deflated it — which `zipfile` does by default — would not read.
    """
    path.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as archive:
        if stored_first is not None:
            info = zipfile.ZipInfo(stored_first, date_time=FIXED_DATE)
            info.compress_type = zipfile.ZIP_STORED
            archive.writestr(info, parts[stored_first])
        for name, body in parts.items():
            if name == stored_first:
                continue
            info = zipfile.ZipInfo(name, date_time=FIXED_DATE)
            info.compress_type = zipfile.ZIP_DEFLATED
            archive.writestr(info, body)
    print(f"wrote {path.relative_to(HERE.parent.parent)} ({path.stat().st_size} bytes)")


# A package whose header and footer carry text this slice does not read. The body is one run, so
# the artifact is small; the point is the DECLARED ERASURE (Anydoc's A14) — three unread text
# parts, counted and named, rather than a body silently returned as if it were the document.
HEADER = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:p><w:r><w:t>Confidential — do not distribute</w:t></w:r></w:p>
</w:hdr>
"""

FOOTER = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:ftr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:p><w:r><w:t>Page number goes here, and this engine does not know which</w:t></w:r></w:p>
</w:ftr>
"""

FOOTNOTES = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:footnotes xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:footnote w:id="1"><w:p><w:r><w:t>A source nobody read.</w:t></w:r></w:p></w:footnote>
</w:footnotes>
"""

BODY_ONLY = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body><w:p><w:r><w:t>The body is all this slice reads.</w:t></w:r></w:p></w:body>
</w:document>
"""

WITH_UNREAD_PARTS = {
    "[Content_Types].xml": with_png(CONTENT_TYPES),
    "_rels/.rels": RELS,
    "word/document.xml": BODY_ONLY,
    "word/header1.xml": HEADER,
    "word/footer1.xml": FOOTER,
    "word/footnotes.xml": FOOTNOTES,
    # v2-S11. TWO embedded assets, and two rather than one on purpose: a count of 1 is the count
    # a reader gets from a great many mistakes, and 2 is not. They carry NO TEXT, which is why
    # they cannot go in the same bucket as the three parts above — the message on that bucket says
    # its parts "carry text", and a PNG does not.
    #
    # Nothing in `word/document.xml` references them. That is deliberate and it is also ordinary:
    # a word processor leaves orphaned media in a package all the time. The count answers "what is
    # in this package that this reader did not read", and an entry's referencedness does not change
    # the answer.
    "word/media/image1.png": PICTURE,
    "word/media/image2.png": PICTURE,
}


# ---------------------------------------------------------------------------
# v2-S3 — workbooks
#
# Authored to the same rules as the documents above, plus two the spreadsheet slice needs:
#
# 1. **The sheet-to-part mapping is not positional.** `Ledger` is `sheet1.xml` and
#    `Notes & sources` is `sheet3.xml` — the shape a real workbook takes after a sheet is
#    deleted. A reader that assumed `xl/worksheets/sheet{N}.xml` in `<sheets>` order would put
#    the second sheet's cells under a part that does not exist, so the fixture forces
#    `xl/_rels/workbook.xml.rels` to actually be read.
# 2. **Rows are numbered by the file, not counted by the reader.** `Ledger` jumps from row 2 to
#    row 12. A reader that incremented a counter would call the last row 3.
# ---------------------------------------------------------------------------

XLSX_MAIN = "http://schemas.openxmlformats.org/spreadsheetml/2006/main"
PKG_RELS = "http://schemas.openxmlformats.org/package/2006/relationships"
OFFICE_RELS = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"

WORKBOOK_CONTENT_TYPES = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
  <Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
  <Override PartName="/xl/worksheets/sheet3.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
  <Override PartName="/xl/sharedStrings.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml"/>
</Types>
"""

WORKBOOK_RELS = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="{PKG_RELS}">
  <Relationship Id="rId1" Type="{OFFICE_RELS}/officeDocument" Target="xl/workbook.xml"/>
</Relationships>
"""

# The sheet name carries an ampersand for the same reason a cell does: an entity dropped in a
# sheet name would be a wrong address rather than wrong text, which is worse.
WORKBOOK = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="{XLSX_MAIN}" xmlns:r="{OFFICE_RELS}">
  <sheets>
    <sheet name="Ledger" sheetId="1" r:id="rId1"/>
    <sheet name="Notes &amp; sources" sheetId="2" r:id="rId7"/>
  </sheets>
</workbook>
"""

WORKBOOK_PART_RELS = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="{PKG_RELS}">
  <Relationship Id="rId1" Type="{OFFICE_RELS}/worksheet" Target="worksheets/sheet1.xml"/>
  <Relationship Id="rId7" Type="{OFFICE_RELS}/worksheet" Target="worksheets/sheet3.xml"/>
  <Relationship Id="rId9" Type="{OFFICE_RELS}/sharedStrings" Target="sharedStrings.xml"/>
</Relationships>
"""

# Index 2 is the rich-text form: one `<si>` whose text is split across two `<r>` runs. A cell
# citing it must get the whole string, because that is the string the workbook shows.
SHARED_STRINGS = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<sst xmlns="{XLSX_MAIN}" count="4" uniqueCount="4">
  <si><t>Rows &amp; columns are S3.</t></si>
  <si><t>Evidence, not extraction.</t></si>
  <si><r><t>A quote binds to a cell</t></r><r><t xml:space="preserve"> and never to a print range.</t></r></si>
  <si><t>Read because the workbook listed it.</t></si>
</sst>
"""

# `A12` is a formula with a cached `<v>`: the node's text is that cached value, as stored, and
# nothing here evaluates anything. `B12` is an empty cell — no value, so no node, exactly as a
# `<w:r>` with no `<w:t>` is no node.
SHEET_LEDGER = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="{XLSX_MAIN}">
  <sheetData>
    <row r="1">
      <c r="A1" t="s"><v>0</v></c>
      <c r="B1" t="s"><v>1</v></c>
    </row>
    <row r="2">
      <c r="A2" t="s"><v>2</v></c>
      <c r="B2"><v>42</v></c>
      <c r="C2" t="inlineStr"><is><t>Inline, not shared.</t></is></c>
    </row>
    <row r="12">
      <c r="A12"><f>SUM(B2:B2)</f><v>42</v></c>
      <c r="B12"/>
    </row>
  </sheetData>
</worksheet>
"""

SHEET_NOTES = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="{XLSX_MAIN}">
  <sheetData>
    <row r="1"><c r="A1" t="s"><v>3</v></c></row>
  </sheetData>
</worksheet>
"""

WORKBOOK_PARTS = {
    "[Content_Types].xml": WORKBOOK_CONTENT_TYPES,
    "_rels/.rels": WORKBOOK_RELS,
    "xl/workbook.xml": WORKBOOK,
    "xl/_rels/workbook.xml.rels": WORKBOOK_PART_RELS,
    "xl/sharedStrings.xml": SHARED_STRINGS,
    "xl/worksheets/sheet1.xml": SHEET_LEDGER,
    "xl/worksheets/sheet3.xml": SHEET_NOTES,
}


# A workbook whose chart, drawing and comments carry text this slice does not read. One sheet,
# one cell, so the artifact is small; the point is the DECLARED ERASURE (Anydoc's A14) — three
# unread text parts, counted and named, rather than a sheet returned as if it were the workbook.
CHART = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<chartSpace xmlns="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <chart><title><tx><rich><a:p xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:r><a:t>Revenue nobody read</a:t></a:r></a:p></rich></tx></title></chart>
</chartSpace>
"""

DRAWING = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing"/>
"""

COMMENTS = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<comments xmlns="{XLSX_MAIN}">
  <commentList><comment ref="A1" authorId="0"><text><t>A remark nobody read.</t></text></comment></commentList>
</comments>
"""

ONE_SHEET_ONLY = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="{XLSX_MAIN}" xmlns:r="{OFFICE_RELS}">
  <sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets>
</workbook>
"""

ONE_SHEET_RELS = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="{PKG_RELS}">
  <Relationship Id="rId1" Type="{OFFICE_RELS}/worksheet" Target="worksheets/sheet1.xml"/>
</Relationships>
"""

ONE_CELL = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="{XLSX_MAIN}">
  <sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>The sheets are all this slice reads.</t></is></c></row></sheetData>
</worksheet>
"""

WORKBOOK_WITH_UNREAD_PARTS = {
    "[Content_Types].xml": with_png(WORKBOOK_CONTENT_TYPES),
    "_rels/.rels": WORKBOOK_RELS,
    "xl/workbook.xml": ONE_SHEET_ONLY,
    "xl/_rels/workbook.xml.rels": ONE_SHEET_RELS,
    "xl/worksheets/sheet1.xml": ONE_CELL,
    "xl/charts/chart1.xml": CHART,
    "xl/drawings/drawing1.xml": DRAWING,
    "xl/comments1.xml": COMMENTS,
    # v2-S11. ONE embedded asset, a different number from the DOCX fixture's two and the deck's
    # three, so a reader that returned another reader's count would be caught by the number alone.
    # `xl/drawings/` above is the drawing XML that POSITIONS a picture and carries text; this is
    # the picture. They are two different erasures and the package puts them in two places.
    "xl/media/image1.png": PICTURE,
}


# ---------------------------------------------------------------------------
# v2-S4 — presentations
#
# Authored to the same rules as the documents and workbooks above, plus the two the slide reader
# needs, both of which came out of measuring 18 real decks:
#
# 1. **The slide-to-part mapping is not positional, and the relationship ids are not either.**
#    The deck's second slide is `slide7.xml` behind `rId4`, so a reader that assumed
#    `ppt/slides/slide{N}.xml` in `<p:sldIdLst>` order — or that sorted by relationship id — puts
#    the wrong slide's text at the wrong address. In a real 55-slide deck `rId13` binds
#    `slides/slide12.xml`, so this is the ordinary case rather than an adversarial one.
# 2. **Shapes nest and shape ids repeat.** Slide one puts a shape inside a `<p:grpSp>` (groups
#    appeared on essentially every slide of every real deck) and gives two shapes the same
#    `<p:cNvPr id>` — which real Open XML SDK output does, and which PowerPoint opens — so the
#    locator cannot be addressing by that id.
# ---------------------------------------------------------------------------

PML = "http://schemas.openxmlformats.org/presentationml/2006/main"
DML = "http://schemas.openxmlformats.org/drawingml/2006/main"

DECK_CONTENT_TYPES = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/slides/slide7.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
</Types>
"""

DECK_RELS = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="{PKG_RELS}">
  <Relationship Id="rId1" Type="{OFFICE_RELS}/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>
"""

# `p:notesSz` is REQUIRED by `CT_Presentation` — measured present in 18/18 real decks — so it is
# here even though nothing reads it. `p:sldSz` is present and deliberately ignored: a slide's
# size in EMUs is not a page, and turning it into one is what this version refuses.
PRESENTATION = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:p="{PML}" xmlns:r="{OFFICE_RELS}">
  <p:sldIdLst>
    <p:sldId id="256" r:id="rId2"/>
    <p:sldId id="257" r:id="rId4"/>
  </p:sldIdLst>
  <p:sldSz cx="12192000" cy="6858000"/>
  <p:notesSz cx="6858000" cy="9144000"/>
</p:presentation>
"""

PRESENTATION_RELS = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="{PKG_RELS}">
  <Relationship Id="rId2" Type="{OFFICE_RELS}/slide" Target="slides/slide1.xml"/>
  <Relationship Id="rId4" Type="{OFFICE_RELS}/slide" Target="slides/slide7.xml"/>
</Relationships>
"""

# Shape 2 is a plain title. Shape 3 lives inside a group and REUSES id 2 — legal enough that
# PowerPoint opens such files, and the reason the locator addresses shapes by position.
SLIDE_ONE = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:p="{PML}" xmlns:a="{DML}"><p:cSld><p:spTree>
  <p:nvGrpSpPr><p:cNvPr id="1" name=""/></p:nvGrpSpPr>
  <p:sp>
    <p:nvSpPr><p:cNvPr id="2" name="Title 1"/></p:nvSpPr>
    <p:txBody>
      <a:p><a:r><a:t>Evidence, not extraction.</a:t></a:r></a:p>
      <a:p><a:r><a:t>Slides &amp; shapes are S4</a:t></a:r><a:r><a:t> and never a page.</a:t></a:r></a:p>
    </p:txBody>
  </p:sp>
  <p:grpSp>
    <p:nvGrpSpPr><p:cNvPr id="9" name="Group 8"/></p:nvGrpSpPr>
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Grouped 2"/></p:nvSpPr>
      <p:txBody><a:p><a:r><a:t>Inside a group, and still read.</a:t></a:r></a:p></p:txBody>
    </p:sp>
  </p:grpSp>
</p:spTree></p:cSld></p:sld>
"""

SLIDE_SEVEN = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:p="{PML}" xmlns:a="{DML}"><p:cSld><p:spTree>
  <p:nvGrpSpPr><p:cNvPr id="1" name=""/></p:nvGrpSpPr>
  <p:sp>
    <p:nvSpPr><p:cNvPr id="4" name="Body 3"/></p:nvSpPr>
    <p:txBody><a:p><a:r><a:t>Read because the deck listed it.</a:t></a:r></a:p></p:txBody>
  </p:sp>
</p:spTree></p:cSld></p:sld>
"""

DECK_PARTS = {
    "[Content_Types].xml": DECK_CONTENT_TYPES,
    "_rels/.rels": DECK_RELS,
    "ppt/presentation.xml": PRESENTATION,
    "ppt/_rels/presentation.xml.rels": PRESENTATION_RELS,
    "ppt/slides/slide1.xml": SLIDE_ONE,
    "ppt/slides/slide7.xml": SLIDE_SEVEN,
}


# A deck whose notes, layout and master carry text this slice does not read, and whose one slide
# holds a table and a slide-number field. Three unread PARTS and two unread SHAPES, counted and
# named — the declared erasure (Anydoc's A14) in both of its halves.
NOTES_SLIDE = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:notes xmlns:p="{PML}" xmlns:a="{DML}"><p:cSld><p:spTree>
  <p:sp><p:nvSpPr><p:cNvPr id="2" name="Notes"/></p:nvSpPr>
    <p:txBody><a:p><a:r><a:t>A speaker note nobody read.</a:t></a:r></a:p></p:txBody></p:sp>
</p:spTree></p:cSld></p:notes>
"""

SLIDE_LAYOUT = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:p="{PML}" xmlns:a="{DML}"><p:cSld name="Title Slide"><p:spTree>
  <p:sp><p:nvSpPr><p:cNvPr id="2" name="Title Placeholder"/></p:nvSpPr>
    <p:txBody><a:p><a:r><a:t>Click to edit Master title style</a:t></a:r></a:p></p:txBody></p:sp>
</p:spTree></p:cSld></p:sldLayout>
"""

SLIDE_MASTER = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:p="{PML}" xmlns:a="{DML}"><p:cSld><p:spTree>
  <p:sp><p:nvSpPr><p:cNvPr id="2" name="Footer"/></p:nvSpPr>
    <p:txBody><a:p><a:r><a:t>Confidential — on every slide, and unread</a:t></a:r></a:p></p:txBody></p:sp>
</p:spTree></p:cSld></p:sldMaster>
"""

SLIDE_WITH_UNREAD_SHAPES = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:p="{PML}" xmlns:a="{DML}"><p:cSld><p:spTree>
  <p:sp>
    <p:nvSpPr><p:cNvPr id="2" name="Title 1"/></p:nvSpPr>
    <p:txBody><a:p><a:r><a:t>The shapes are all this slice reads.</a:t></a:r></a:p></p:txBody>
  </p:sp>
  <p:graphicFrame>
    <p:nvGraphicFramePr><p:cNvPr id="5" name="Table 4"/></p:nvGraphicFramePr>
    <a:graphic><a:graphicData><a:tbl><a:tr><a:tc><a:txBody>
      <a:p><a:r><a:t>A table cell nobody read.</a:t></a:r></a:p>
    </a:txBody></a:tc></a:tr></a:tbl></a:graphicData></a:graphic>
  </p:graphicFrame>
  <p:sp>
    <p:nvSpPr><p:cNvPr id="6" name="Slide Number Placeholder 5"/></p:nvSpPr>
    <p:txBody><a:p><a:fld id="{{C51EAA63-D034-42AE-91FA-B13B9518C7BE}}" type="slidenum"><a:t>1</a:t></a:fld></a:p></p:txBody>
  </p:sp>
</p:spTree></p:cSld></p:sld>
"""

ONE_SLIDE_ONLY = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:p="{PML}" xmlns:r="{OFFICE_RELS}">
  <p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst>
  <p:notesSz cx="6858000" cy="9144000"/>
</p:presentation>
"""

ONE_SLIDE_RELS = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="{PKG_RELS}">
  <Relationship Id="rId2" Type="{OFFICE_RELS}/slide" Target="slides/slide1.xml"/>
</Relationships>
"""

DECK_WITH_UNREAD_PARTS = {
    "[Content_Types].xml": with_png(DECK_CONTENT_TYPES),
    "_rels/.rels": DECK_RELS,
    "ppt/presentation.xml": ONE_SLIDE_ONLY,
    "ppt/_rels/presentation.xml.rels": ONE_SLIDE_RELS,
    "ppt/slides/slide1.xml": SLIDE_WITH_UNREAD_SHAPES,
    "ppt/notesSlides/notesSlide1.xml": NOTES_SLIDE,
    "ppt/slideLayouts/slideLayout1.xml": SLIDE_LAYOUT,
    "ppt/slideMasters/slideMaster1.xml": SLIDE_MASTER,
    # v2-S11. THREE embedded assets — a presentation is the format that carries the most of them,
    # and three is a third distinct number across the three OOXML fixtures.
    "ppt/media/image1.png": PICTURE,
    "ppt/media/image2.png": PICTURE,
    "ppt/media/image3.png": PICTURE,
}


# ---------------------------------------------------------------------------
# v2-S5 — OpenDocument text
#
# The first fixture family here that is not OOXML, and it is authored to three rules the ODF
# readers need on top of the ones above:
#
# 1. **`mimetype` is first and stored.** The OpenDocument package specification requires it, so
#    that a consumer can identify the document from the leading bytes. `write(..., stored_first=)`
#    is what makes `zipfile` do it — its default would deflate the entry and put it wherever the
#    dict order fell, and the reader would refuse the package. This is also what keeps an `.ods`
#    and an `.odp` from ever being claimed as text.
# 2. **The page is in the file, and is not read.** `content.xml` carries a
#    `<text:soft-page-break/>` and `styles.xml` carries an `fo:page-width` — between them a
#    `PageRecord` would need no arithmetic at all. The fixture contains both precisely so the test
#    that `pages` is empty is not vacuous.
# 3. **The counter advances through what is not read.** The footnote and the comment in the second
#    package each hold a `<text:p>`, so the blocks after them are numbered 3 and 5. A reader that
#    counted only what it kept would call them 2 and 3, and every citation after a footnote would
#    land on the wrong paragraph.
# ---------------------------------------------------------------------------

ODF_OFFICE = "urn:oasis:names:tc:opendocument:xmlns:office:1.0"
ODF_TEXT = "urn:oasis:names:tc:opendocument:xmlns:text:1.0"
ODF_TABLE = "urn:oasis:names:tc:opendocument:xmlns:table:1.0"
ODF_STYLE = "urn:oasis:names:tc:opendocument:xmlns:style:1.0"
ODF_FO = "urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0"
ODF_MANIFEST = "urn:oasis:names:tc:opendocument:xmlns:manifest:1.0"
ODF_META = "urn:oasis:names:tc:opendocument:xmlns:meta:1.0"
DC = "http://purl.org/dc/elements/1.1/"

ODT_MIMETYPE = "application/vnd.oasis.opendocument.text"

# Six blocks that become nodes and one that does not. The `<text:p/>` is block 3 and carries no
# text, so it is no node — and the block after it is still block 4, which is what a consumer
# counting elements in this file would find. The ampersand is here for the reason it is in every
# other fixture in this file, the span is here so the test that a sentence is ONE node rather than
# three is not vacuous, and the paragraph is indented across three source lines so the test that
# ODF's own whitespace rule is applied has something to apply it to.
ODT_CONTENT = f"""<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="{ODF_OFFICE}" xmlns:text="{ODF_TEXT}" xmlns:table="{ODF_TABLE}" office:version="1.3">
  <office:body>
    <office:text>
      <text:h text:style-name="Heading_20_1" text:outline-level="1">Evidence, not extraction.</text:h>
      <text:p text:style-name="Standard">A quote binds to a <text:span text:style-name="T1">paragraph</text:span>
        and never
        to a page.</text:p>
      <text:p/>
      <text:p>Rows &amp; columns<text:tab/>are tabbed.</text:p>
      <text:p>Three spaces:<text:s text:c="3"/>stated, not measured.</text:p>
      <text:p>Split by the producer<text:soft-page-break/> and rejoined here.</text:p>
      <table:table table:name="Ledger">
        <table:table-row>
          <table:table-cell office:value-type="string"><text:p>In a cell, and still a paragraph.</text:p></table:table-cell>
        </table:table-row>
      </table:table>
    </office:text>
  </office:body>
</office:document-content>
"""

ODT_MANIFEST = f"""<?xml version="1.0" encoding="UTF-8"?>
<manifest:manifest xmlns:manifest="{ODF_MANIFEST}" manifest:version="1.3">
  <manifest:file-entry manifest:full-path="/" manifest:version="1.3" manifest:media-type="{ODT_MIMETYPE}"/>
  <manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/>
</manifest:manifest>
"""

# Deliberately only the three entries the reader consumes, so the clean package declares NO
# erasure — the same shape `simple-paragraphs` has for a DOCX. A package a word processor wrote
# would carry styles, metadata and settings as well, which is what the second fixture is.
ODT_PARTS = {
    "mimetype": ODT_MIMETYPE,
    "META-INF/manifest.xml": ODT_MANIFEST,
    "content.xml": ODT_CONTENT,
}


# A package whose styles, metadata and picture are not read, and whose content part holds a
# footnote and a comment that are not read either. Both halves of the declared erasure (Anydoc's
# A14) in one document — three unread ENTRIES and two unread REGIONS.
#
# `styles.xml` is where an ODF header lives, and it is also where `fo:page-width` lives. Both are
# in this fixture on purpose: the header is text a caller could otherwise conclude is absent, and
# the page width is the number a reader looking for a `PageRecord` would reach for.
ODT_STYLES = f"""<?xml version="1.0" encoding="UTF-8"?>
<office:document-styles xmlns:office="{ODF_OFFICE}" xmlns:style="{ODF_STYLE}" xmlns:text="{ODF_TEXT}" xmlns:fo="{ODF_FO}" office:version="1.3">
  <office:automatic-styles>
    <style:page-layout style:name="pm1">
      <style:page-layout-properties fo:page-width="21.001cm" fo:page-height="29.7cm"/>
    </style:page-layout>
  </office:automatic-styles>
  <office:master-styles>
    <style:master-page style:name="Standard" style:page-layout-name="pm1">
      <style:header><text:p>Confidential — on every page, and unread</text:p></style:header>
    </style:master-page>
  </office:master-styles>
</office:document-styles>
"""

ODT_META = f"""<?xml version="1.0" encoding="UTF-8"?>
<office:document-meta xmlns:office="{ODF_OFFICE}" xmlns:meta="{ODF_META}" xmlns:dc="{DC}" office:version="1.3">
  <office:meta><dc:title>A title nobody read</dc:title></office:meta>
</office:document-meta>
"""

ODT_BODY_WITH_REGIONS = f"""<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="{ODF_OFFICE}" xmlns:text="{ODF_TEXT}" xmlns:dc="{DC}" office:version="1.3">
  <office:body>
    <office:text>
      <text:p>The body is all this slice reads.<text:note text:id="ftn1" text:note-class="footnote"><text:note-citation>1</text:note-citation><text:note-body><text:p>A source nobody read.</text:p></text:note-body></text:note></text:p>
      <text:p>Reviewed<office:annotation><dc:creator>A reviewer</dc:creator><text:p>A remark nobody read.</text:p></office:annotation> and unchanged.</text:p>
      <text:p>After both, and still block five.</text:p>
    </office:text>
  </office:body>
</office:document-content>
"""

ODT_MANIFEST_WITH_PARTS = f"""<?xml version="1.0" encoding="UTF-8"?>
<manifest:manifest xmlns:manifest="{ODF_MANIFEST}" manifest:version="1.3">
  <manifest:file-entry manifest:full-path="/" manifest:version="1.3" manifest:media-type="{ODT_MIMETYPE}"/>
  <manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/>
  <manifest:file-entry manifest:full-path="styles.xml" manifest:media-type="text/xml"/>
  <manifest:file-entry manifest:full-path="meta.xml" manifest:media-type="text/xml"/>
  <manifest:file-entry manifest:full-path="Pictures/10000000.png" manifest:media-type="image/png"/>
</manifest:manifest>
"""

# A 1x1 PNG, authored here rather than sampled: the point is that a package entry holding
# something this reader cannot read is COUNTED, and the smallest valid file makes that point.
ODT_PICTURE = PICTURE

ODT_WITH_UNREAD_PARTS = {
    "mimetype": ODT_MIMETYPE,
    "META-INF/manifest.xml": ODT_MANIFEST_WITH_PARTS,
    "content.xml": ODT_BODY_WITH_REGIONS,
    "styles.xml": ODT_STYLES,
    "meta.xml": ODT_META,
    "Pictures/10000000.png": ODT_PICTURE,
}


# ---------------------------------------------------------------------------------------
# v2-S6 — OpenDocument spreadsheet
#
# The container is v2-S5's, unchanged: `mimetype` first and stored, a manifest that declares
# `content.xml`. What is authored here is the vocabulary above it, to four rules:
#
# 1. **The address is a position, and the file compresses it.** `table:number-columns-repeated`
#    is how ODF states a run of cells, so the clean package writes one — a reader that ignored it
#    would put `Total` at column 3 instead of column 5, and the test asserts 5.
# 2. **The ampersand is in a cell AND in a table name.** A `&` dropped from a cell is wrong text;
#    one dropped from `table:name` is a wrong ADDRESS, which is the worse of the two.
# 3. **The page is in the file, and is not read.** `content.xml` carries a
#    `<text:soft-page-break/>` inside a cell's own paragraph and `styles.xml` carries an
#    `fo:page-width`, so the test that `pages` is empty is not vacuous.
# 4. **Every leak the ODT reader's review found has a byte-level cousin here**, in the second
#    package: an image title and description, an embedded object's base64, a generated number, a
#    field's cached page count and page number, and ruby guide text. Each is asserted ABSENT from
#    `Node.text` by inflating this package's own `content.xml` — never by grepping this file,
#    which is how v2-S5's first version of that guard passed on a `#` comment.
# ---------------------------------------------------------------------------------------

ODF_SVG = "urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0"
ODF_DRAW = "urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
XHTML = "http://www.w3.org/1999/xhtml"
MATHML = "http://www.w3.org/1998/Math/MathML"

ODS_MIMETYPE = "application/vnd.oasis.opendocument.spreadsheet"

# One table, six cells that become nodes and several that do not. The empty run of three columns
# is the load-bearing part: `Total` is at column 5 because the file says three cells sit between.
ODS_CONTENT = f"""<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="{ODF_OFFICE}" xmlns:text="{ODF_TEXT}" xmlns:table="{ODF_TABLE}" office:version="1.3">
  <office:body>
    <office:spreadsheet>
      <table:table table:name="Rows &amp; Columns">
        <table:table-column table:number-columns-repeated="5"/>
        <table:table-row>
          <table:table-cell office:value-type="string"><text:p>Evidence, not extraction.</text:p></table:table-cell>
          <table:table-cell table:number-columns-repeated="3"/>
          <table:table-cell office:value-type="string"><text:p>Total</text:p></table:table-cell>
        </table:table-row>
        <table:table-row>
          <table:table-cell office:value-type="float" office:value="42"><text:p>42</text:p></table:table-cell>
          <table:table-cell office:value-type="percentage" office:value="0.25"><text:p>25%</text:p></table:table-cell>
          <table:table-cell office:value-type="string"><text:p>Rows &amp; columns<text:tab/>are tabbed.</text:p></table:table-cell>
          <table:table-cell office:value-type="string"><text:p>Three spaces:<text:s text:c="3"/>stated, not measured.</text:p></table:table-cell>
          <table:table-cell table:formula="of:=SUM([.A2:.A2])" office:value-type="float" office:value="42"><text:p>42</text:p></table:table-cell>
        </table:table-row>
        <table:table-row>
          <table:table-cell table:number-columns-spanned="2" office:value-type="string"><text:p>Split by the producer<text:soft-page-break/> and rejoined here.</text:p></table:table-cell>
          <table:covered-table-cell/>
          <table:table-cell office:value-type="string"><text:p>First line</text:p><text:p>Second line</text:p></table:table-cell>
        </table:table-row>
      </table:table>
    </office:spreadsheet>
  </office:body>
</office:document-content>
"""

ODS_MANIFEST = f"""<?xml version="1.0" encoding="UTF-8"?>
<manifest:manifest xmlns:manifest="{ODF_MANIFEST}" manifest:version="1.3">
  <manifest:file-entry manifest:full-path="/" manifest:version="1.3" manifest:media-type="{ODS_MIMETYPE}"/>
  <manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/>
</manifest:manifest>
"""

# Only the three entries the reader consumes, so the clean package declares NO erasure.
ODS_PARTS = {
    "mimetype": ODS_MIMETYPE,
    "META-INF/manifest.xml": ODS_MANIFEST,
    "content.xml": ODS_CONTENT,
}


# The package that declares erasures, both kinds. Three unread ENTRIES and, inside `content.xml`,
# a comment, a page-anchored shape, a second framed rendition, and six foreign leaks.
#
# The two renditions in one `<draw:frame>` are what v2-S5 owed this slice: it wrote the
# first-rendition-wins rule off the specification and recorded that it had never been measured
# against a document that nests two. This is that document.
ODS_CONTENT_WITH_UNREAD = f"""<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="{ODF_OFFICE}" xmlns:text="{ODF_TEXT}" xmlns:table="{ODF_TABLE}" xmlns:draw="{ODF_DRAW}" xmlns:svg="{ODF_SVG}" xmlns:xhtml="{XHTML}" xmlns:math="{MATHML}" xmlns:dc="{DC}" office:version="1.3">
  <office:body>
    <office:spreadsheet>
      <table:table table:name="Ledger">
        <table:shapes>
          <draw:frame><draw:text-box><text:p>FLOATING-SHAPE-TEXT</text:p></draw:text-box></draw:frame>
        </table:shapes>
        <table:table-row>
          <table:table-cell office:value-type="string">
            <text:p>Kept sentence.</text:p>
            <office:annotation><dc:creator>Reviewer</dc:creator><text:p>COMMENT-BODY</text:p></office:annotation>
          </table:table-cell>
          <table:table-cell office:value-type="string">
            <text:p>Framed:<draw:frame>
              <draw:text-box><text:p>FIRST-RENDITION</text:p></draw:text-box>
              <draw:text-box><text:p>SECOND-RENDITION</text:p></draw:text-box>
            </draw:frame></text:p>
          </table:table-cell>
        </table:table-row>
        <table:table-row>
          <table:table-cell office:value-type="string">
            <text:p>Leaks:<draw:frame><draw:image><svg:title>IMAGE-TITLE</svg:title><svg:desc>IMAGE-DESC</svg:desc></draw:image></draw:frame></text:p>
          </table:table-cell>
          <table:table-cell office:value-type="string">
            <text:p>Object:<draw:object-ole><office:binary-data>QkFTRTY0LURBVEE=</office:binary-data></draw:object-ole></text:p>
          </table:table-cell>
          <table:table-cell office:value-type="string">
            <text:p><text:number>2.1</text:number>Numbered heading label</text:p>
          </table:table-cell>
          <table:table-cell office:value-type="string">
            <text:p>Fields:<text:page-count>17</text:page-count><text:page-number>4</text:page-number></text:p>
          </table:table-cell>
          <table:table-cell office:value-type="string">
            <text:p><text:ruby><text:ruby-base>kanji</text:ruby-base><text:ruby-text>RUBY-GUIDE</text:ruby-text></text:ruby></text:p>
          </table:table-cell>
        </table:table-row>
        <table:table-row>
          <table:table-cell office:value-type="string">
            <text:p>Namespaces.</text:p>
            <xhtml:table><xhtml:p>XHTML-STOLEN</xhtml:p></xhtml:table>
          </table:table-cell>
          <table:table-cell office:value-type="string">
            <text:p>Formula source:<math:annotation>MATHML-ANNOTATION</math:annotation></text:p>
          </table:table-cell>
        </table:table-row>
      </table:table>
    </office:spreadsheet>
  </office:body>
</office:document-content>
"""

# `fo:page-width` lives here, and is not read. Together with the `<text:soft-page-break/>` in the
# clean package's own content it is why "no pages" is a refusal rather than an absence.
ODS_STYLES = f"""<?xml version="1.0" encoding="UTF-8"?>
<office:document-styles xmlns:office="{ODF_OFFICE}" xmlns:style="{ODF_STYLE}" xmlns:fo="{ODF_FO}" office:version="1.3">
  <office:automatic-styles>
    <style:page-layout style:name="Mpm1">
      <style:page-layout-properties fo:page-width="8.5in" fo:page-height="11in"/>
    </style:page-layout>
  </office:automatic-styles>
</office:document-styles>
"""

ODS_META = f"""<?xml version="1.0" encoding="UTF-8"?>
<office:document-meta xmlns:office="{ODF_OFFICE}" xmlns:meta="{ODF_META}" xmlns:dc="{DC}" office:version="1.3">
  <office:meta><dc:title>UNREAD-METADATA-TITLE</dc:title></office:meta>
</office:document-meta>
"""

ODS_SETTINGS = f"""<?xml version="1.0" encoding="UTF-8"?>
<office:document-settings xmlns:office="{ODF_OFFICE}" office:version="1.3"/>
"""

ODS_MANIFEST_WITH_PARTS = f"""<?xml version="1.0" encoding="UTF-8"?>
<manifest:manifest xmlns:manifest="{ODF_MANIFEST}" manifest:version="1.3">
  <manifest:file-entry manifest:full-path="/" manifest:version="1.3" manifest:media-type="{ODS_MIMETYPE}"/>
  <manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/>
  <manifest:file-entry manifest:full-path="styles.xml" manifest:media-type="text/xml"/>
  <manifest:file-entry manifest:full-path="meta.xml" manifest:media-type="text/xml"/>
  <manifest:file-entry manifest:full-path="settings.xml" manifest:media-type="text/xml"/>
</manifest:manifest>
"""

ODS_WITH_UNREAD_PARTS = {
    "mimetype": ODS_MIMETYPE,
    "META-INF/manifest.xml": ODS_MANIFEST_WITH_PARTS,
    "content.xml": ODS_CONTENT_WITH_UNREAD,
    "styles.xml": ODS_STYLES,
    "meta.xml": ODS_META,
    "settings.xml": ODS_SETTINGS,
}


# ---------------------------------------------------------------------------------------
# v2-S7 — OpenDocument presentation
#
# The container is v2-S5's, unchanged for the third time: `mimetype` first and stored, a manifest
# that declares `content.xml`. What is authored here is the vocabulary above it, to five rules:
#
# 1. **The draw pages are really there, so `pages: []` is a refusal rather than an absence.** The
#    clean package lists TWO `<draw:page>` elements, each with a `draw:name`, and the second
#    package's `styles.xml` carries a `<style:master-page>` with `fo:page-width`. Between them a
#    PageRecord would need no arithmetic at all — which is exactly the temptation the test
#    asserting an empty `pages` exists to pin.
# 2. **The address must not turn on serialization.** Page two opens with `<draw:frame/>`, so the
#    shape that follows it is shape 2 — a reader that skipped the self-closing form would say 1.
# 3. **The ampersand is in visible text AND in both `draw:name`s.** A `&` dropped from text is
#    wrong text; one dropped from a name is a wrong LABEL, which is what a person reading a
#    citation matches against the original package.
# 4. **The frame-alternative rule is expressible here, and the answer is not the spreadsheet's.**
#    The second package nests two `<draw:text-box>` children in one `<draw:frame>` and puts a
#    self-closing one first in another — the two constructs v2-S5 and v2-S6 each had to author by
#    hand because no ODF producer was available, authored again because none is available now.
# 5. **Every allowlist leak v2-S5 and v2-S6 listed has a byte-level cousin here**, plus the two
#    this format adds: an image title inside a shape with NO `<text:p>` anywhere in it, and a
#    speaker-notes body. Each is asserted ABSENT from `Node.text` by inflating this package's own
#    `content.xml` — never by grepping this file.
# ---------------------------------------------------------------------------------------

ODF_PRESENTATION = "urn:oasis:names:tc:opendocument:xmlns:presentation:1.0"

ODP_MIMETYPE = "application/vnd.oasis.opendocument.presentation"

# Two draw pages, five shapes, seven blocks. `<text:soft-page-break/>` is inside a shape's own
# paragraph so that the "a break is not a page" assertion has something to be about, and the
# self-closing `<draw:frame/>` on page two is what makes the shape index measurable.
ODP_CONTENT = f"""<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="{ODF_OFFICE}" xmlns:text="{ODF_TEXT}" xmlns:draw="{ODF_DRAW}" xmlns:presentation="{ODF_PRESENTATION}" office:version="1.3">
  <office:body>
    <office:presentation>
      <draw:page draw:name="Rows &amp; Columns" draw:master-page-name="Default">
        <draw:frame draw:name="Title 1" presentation:class="title">
          <draw:text-box><text:h>Evidence, not extraction.</text:h></draw:text-box>
        </draw:frame>
        <draw:frame draw:name="Body &amp; bullets" presentation:class="outline">
          <draw:text-box>
            <text:p>Rows &amp; columns bind to a shape<text:tab/>and never to a page.</text:p>
            <text:p>Three spaces:<text:s text:c="3"/>stated, not measured.</text:p>
            <text:p>Split by the producer<text:soft-page-break/> and rejoined here.</text:p>
          </draw:text-box>
        </draw:frame>
      </draw:page>
      <draw:page draw:name="Detail">
        <draw:frame/>
        <draw:frame draw:name="Title 2">
          <draw:text-box><text:h>The second draw page</text:h></draw:text-box>
        </draw:frame>
        <draw:custom-shape draw:name="Arrow">
          <text:p>Drawn shapes carry their blocks directly.</text:p>
        </draw:custom-shape>
      </draw:page>
    </office:presentation>
  </office:body>
</office:document-content>
"""

ODP_MANIFEST = f"""<?xml version="1.0" encoding="UTF-8"?>
<manifest:manifest xmlns:manifest="{ODF_MANIFEST}" manifest:version="1.3">
  <manifest:file-entry manifest:full-path="/" manifest:version="1.3" manifest:media-type="{ODP_MIMETYPE}"/>
  <manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/>
</manifest:manifest>
"""

# Only the three entries the reader consumes, so the clean package declares NO erasure.
ODP_PARTS = {
    "mimetype": ODP_MIMETYPE,
    "META-INF/manifest.xml": ODP_MANIFEST,
    "content.xml": ODP_CONTENT,
}


# The package that declares erasures, all three kinds. Three unread ENTRIES and, inside
# `content.xml`, speaker notes, two framed renditions, a self-closing first rendition, an unnamed
# drawing shape, a comment, and the foreign leaks.
ODP_CONTENT_WITH_UNREAD = f"""<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="{ODF_OFFICE}" xmlns:text="{ODF_TEXT}" xmlns:draw="{ODF_DRAW}" xmlns:presentation="{ODF_PRESENTATION}" xmlns:svg="{ODF_SVG}" xmlns:xhtml="{XHTML}" xmlns:math="{MATHML}" xmlns:dc="{DC}" office:version="1.3">
  <office:body>
    <office:presentation>
      <draw:page draw:name="Leaks">
        <draw:frame draw:name="Kept">
          <draw:text-box>
            <text:p>Kept sentence.</text:p>
            <text:p>Reviewed<office:annotation><dc:creator>Reviewer</dc:creator><text:p>COMMENT-BODY</text:p></office:annotation> and unchanged.</text:p>
          </draw:text-box>
        </draw:frame>
        <draw:frame draw:name="Two renditions">
          <draw:text-box><text:p>FIRST-RENDITION</text:p></draw:text-box>
          <draw:text-box><text:p>SECOND-RENDITION</text:p></draw:text-box>
        </draw:frame>
        <draw:frame draw:name="Empty first">
          <draw:text-box/>
          <draw:text-box><text:p>AFTER-AN-EMPTY-FIRST</text:p></draw:text-box>
        </draw:frame>
        <draw:frame draw:name="Picture">
          <draw:image><svg:title>IMAGE-TITLE</svg:title><svg:desc>IMAGE-DESC</svg:desc></draw:image>
        </draw:frame>
        <draw:rect draw:name="Unread shape"><text:p>RECT-TEXT</text:p></draw:rect>
        <presentation:notes>
          <draw:frame draw:name="Notes">
            <draw:text-box><text:p>SPOKEN-ALOUD</text:p></draw:text-box>
          </draw:frame>
        </presentation:notes>
      </draw:page>
      <draw:page draw:name="Foreign">
        <draw:frame draw:name="Fields">
          <draw:text-box>
            <text:p>Object:<draw:object-ole><office:binary-data>QkFTRTY0LURBVEE=</office:binary-data></draw:object-ole></text:p>
            <text:p><text:number>2.1</text:number>Numbered heading label</text:p>
            <text:p>Fields:<text:page-count>17</text:page-count><text:page-number>4</text:page-number></text:p>
            <text:p><text:ruby><text:ruby-base>kanji</text:ruby-base><text:ruby-text>RUBY-GUIDE</text:ruby-text></text:ruby></text:p>
          </draw:text-box>
        </draw:frame>
        <draw:frame draw:name="Namespaces">
          <draw:text-box>
            <text:p>Namespaces.</text:p>
            <xhtml:p>XHTML-STOLEN</xhtml:p>
            <text:p>Formula source:<math:annotation>MATHML-ANNOTATION</math:annotation></text:p>
          </draw:text-box>
        </draw:frame>
      </draw:page>
    </office:presentation>
  </office:body>
</office:document-content>
"""

# `fo:page-width` and a `<style:master-page>` live here, and neither is read. Together with the
# TWO `<draw:page>` elements in the clean package's own content, this is why "no pages" is a
# refusal rather than an absence: a PageRecord was available off these two facts alone.
ODP_STYLES = f"""<?xml version="1.0" encoding="UTF-8"?>
<office:document-styles xmlns:office="{ODF_OFFICE}" xmlns:style="{ODF_STYLE}" xmlns:text="{ODF_TEXT}" xmlns:draw="{ODF_DRAW}" xmlns:fo="{ODF_FO}" office:version="1.3">
  <office:automatic-styles>
    <style:page-layout style:name="PM1">
      <style:page-layout-properties fo:page-width="28cm" fo:page-height="15.75cm"/>
    </style:page-layout>
  </office:automatic-styles>
  <office:master-styles>
    <style:master-page style:name="Default" style:page-layout-name="PM1">
      <draw:frame draw:name="Master title"><draw:text-box><text:p>MASTER-PAGE-TEXT</text:p></draw:text-box></draw:frame>
    </style:master-page>
  </office:master-styles>
</office:document-styles>
"""

ODP_META = f"""<?xml version="1.0" encoding="UTF-8"?>
<office:document-meta xmlns:office="{ODF_OFFICE}" xmlns:meta="{ODF_META}" xmlns:dc="{DC}" office:version="1.3">
  <office:meta><dc:title>UNREAD-METADATA-TITLE</dc:title></office:meta>
</office:document-meta>
"""

ODP_SETTINGS = f"""<?xml version="1.0" encoding="UTF-8"?>
<office:document-settings xmlns:office="{ODF_OFFICE}" office:version="1.3"/>
"""

ODP_MANIFEST_WITH_PARTS = f"""<?xml version="1.0" encoding="UTF-8"?>
<manifest:manifest xmlns:manifest="{ODF_MANIFEST}" manifest:version="1.3">
  <manifest:file-entry manifest:full-path="/" manifest:version="1.3" manifest:media-type="{ODP_MIMETYPE}"/>
  <manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/>
  <manifest:file-entry manifest:full-path="styles.xml" manifest:media-type="text/xml"/>
  <manifest:file-entry manifest:full-path="meta.xml" manifest:media-type="text/xml"/>
  <manifest:file-entry manifest:full-path="settings.xml" manifest:media-type="text/xml"/>
</manifest:manifest>
"""

ODP_WITH_UNREAD_PARTS = {
    "mimetype": ODP_MIMETYPE,
    "META-INF/manifest.xml": ODP_MANIFEST_WITH_PARTS,
    "content.xml": ODP_CONTENT_WITH_UNREAD,
    "styles.xml": ODP_STYLES,
    "meta.xml": ODP_META,
    "settings.xml": ODP_SETTINGS,
}


# ---------------------------------------------------------------------------------------
# v2-S8 — Rich Text Format
#
# **Not a package**, so nothing above is reused: no ZIP, no manifest, no parts. These are written
# as bytes with `write_stream` below, to four rules:
#
# 1. **The magic is the whole of detection.** Both files begin `{\rtf1`, and the tests assert a
#    bare `{`, an OLE compound header and a ZIP are each NOT claimed.
# 2. **The page is in the file, and is not read.** The clean stream carries `\paperw12240` and a
#    `\page` between two paragraphs, so the test that `pages` is empty is not vacuous.
# 3. **Every destination class has a byte-level cousin in the second file**: a font table, a
#    colour table, a style sheet, document info, a picture with `\bin`, a header, a footer, a
#    footnote, a field's instruction AND its cached result, and a `{\*\…}` a reader may ignore.
#    Each is asserted ABSENT from `Node.text` by reading THIS FILE — never by grepping this
#    generator, which is how v2-S5's first version of that guard passed on a `#` comment.
# 4. **The characters are the file's own.** `\'26` is an ampersand below 0x80 and is read; `\'e9`
#    is above it and is DECLARED, because its meaning depends on a code page this reader does not
#    carry a table for. `\u233` says the same character unambiguously and is read.
# ---------------------------------------------------------------------------------------


def write_stream(path: pathlib.Path, body: str) -> None:
    """Write an RTF stream verbatim, as bytes.

    No container and no timestamp, so determinism costs nothing here — unlike the ZIP formats
    above, where `zipfile` stamps a date into every entry.
    """
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(body.encode("ascii"))
    print(f"wrote {path.relative_to(HERE.parent.parent)} ({path.stat().st_size} bytes)")


# Five paragraphs, one of them a pair of table cells. `\paperw` and `\page` are both here so the
# empty `pages` vector is a refusal rather than an absence.
#
# **No `{\fonttbl}` here, deliberately.** Every real producer writes one and the second stream has
# it; this one carries only what the reader consumes, so it declares NO erasure — the same shape
# `simple-paragraphs` has for a DOCX, and what makes the erasure count testable at all.
RTF_PLAIN = (
    r"{\rtf1\ansi\ansicpg1252\paperw12240\paperh15840"
    "\n"
    r"\pard Evidence, not extraction.\par "
    r"Rows \'26 columns bind to a paragraph\tab and never to a page.\par "
    r"A quote binds to the {\b displayed} text, caf\u233 ? included.\par "
    r"\page Split by the producer.\par "
    r"\trowd\intbl Left cell\cell Right cell\cell\row "
    "}"
)


# The stream that declares erasures. Every destination class this reader passes over, plus the two
# byte classes it cannot decode.
RTF_UNREAD = (
    r"{\rtf1\ansi\ansicpg1252\deff0"
    r"{\fonttbl{\f0\froman FONT-TABLE-NAME;}}"
    r"{\colortbl;\red0\green0\blue0;}"
    r"{\stylesheet{\s0 STYLE-SHEET-NAME;}}"
    r"{\info{\title INFO-TITLE}{\author INFO-AUTHOR}}"
    r"{\*\generator IGNORABLE-GENERATOR 1.0;}"
    "\n"
    r"\pard Kept sentence.\par "
    r"{\header HEADER-TEXT\par }"
    r"{\footer FOOTER-TEXT\par }"
    r"Footnoted{\footnote FOOTNOTE-BODY} and kept.\par "
    r"Page {\field{\*\fldinst PAGE }{\fldrslt FIELD-RESULT}} of many.\par "
    r"{\pict\pngblip\picw1\pich1\bin4 {}\a}"
    r"A picture sat above this line.\par "
    r"Undecodable: caf\'e9 and r\'e9sum\'e9.\par "
    r"Decodable: \'26 and \u233 ? are both read."
    "}"
)

# ---------------------------------------------------------------------------------------
# v2-S9 — EPUB
#
# OCF again, and a different family. The container work is v2-S5's — `mimetype` first and stored —
# and everything above it is new: `META-INF/container.xml` names a package document, the package
# document's manifest maps ids to hrefs, and its spine states the reading order. Five rules:
#
# 1. **The spine is the reading order, and the ZIP is not.** The archive stores `aa-second.xhtml`
#    BEFORE `zz-first.xhtml`, and the spine lists `zz-first` first. A reader that took the XHTML
#    entries in central-directory order — or sorted them by name — would put chapter two first.
#    Both orderings are asserted from the package's own bytes, so the test cannot pass by accident.
# 2. **The href is relative to the package document's directory.** Everything lives under `OEBPS/`
#    and the manifest writes bare file names, so a reader that took the href verbatim finds nothing.
# 3. **The page is nameable here and is not read.** The second package carries an EPUB 3 navigation
#    document with a `page-list` — actual print page numbers, written down — so the test that
#    `pages` is empty is a refusal rather than an absence.
# 4. **`linear="no"` is read and labelled.** The clean package's third spine item is auxiliary
#    content; dropping it would lose text the book contains, and reading it unlabelled would be a
#    silent extra.
# 5. **Every skipped class has a byte-level cousin** in the second package: a script, a style
#    sheet, a `<nav>`, an inline SVG title, a MathML annotation, a non-XHTML spine item, an unread
#    image, and a second rendition. Each is asserted ABSENT from `Node.text` by inflating THAT
#    PACKAGE — never by grepping this generator.
# ---------------------------------------------------------------------------------------

XHTML_NS = "http://www.w3.org/1999/xhtml"
OPF_NS = "http://www.idpf.org/2007/opf"
OCF_NS = "urn:oasis:names:tc:opendocument:xmlns:container"
EPUB_NS = "http://www.idpf.org/2007/ops"
SVG_NS = "http://www.w3.org/2000/svg"

EPUB_MIMETYPE = "application/epub+zip"

EPUB_CONTAINER = f"""<?xml version="1.0" encoding="UTF-8"?>
<container xmlns="{OCF_NS}" version="1.0">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>
"""

# The spine lists `zz-first` before `aa-second`; `write()` stores them the other way round.
EPUB_PACKAGE = f"""<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="{OPF_NS}" version="3.0" unique-identifier="pub-id">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:identifier id="pub-id">urn:uuid:0d3f0f4a-0000-4000-8000-000000000001</dc:identifier>
    <dc:title>Rows &amp; Columns</dc:title>
    <dc:language>en</dc:language>
  </metadata>
  <manifest>
    <item id="c1" href="zz-first.xhtml" media-type="application/xhtml+xml"/>
    <item id="c2" href="aa-second.xhtml" media-type="application/xhtml+xml"/>
    <item id="note" href="notes.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine>
    <itemref idref="c1"/>
    <itemref idref="c2"/>
    <itemref idref="note" linear="no"/>
  </spine>
</package>
"""

EPUB_CHAPTER_ONE = f"""<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="{XHTML_NS}">
  <head><title>A title nobody reads as body text</title></head>
  <body>
    <h1>Evidence, not extraction.</h1>
    <p>Rows &amp; columns bind to a block<br/>and never to a page.</p>
    <p>A quote binds to the <em>displayed</em> text, caf&#233; included.</p>
    <pre>  two leading spaces
  and a second line</pre>
    <table><tr><td>Left cell</td><td>Right cell</td></tr></table>
  </body>
</html>
"""

EPUB_CHAPTER_TWO = f"""<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="{XHTML_NS}">
  <head><title>Chapter two</title></head>
  <body>
    <h1>The second spine item</h1>
    <ul><li>First bullet</li><li>Second bullet</li></ul>
  </body>
</html>
"""

EPUB_NOTES = f"""<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="{XHTML_NS}">
  <head><title>Notes</title></head>
  <body>
    <p>Auxiliary content the spine marks non-linear.</p>
  </body>
</html>
"""

# Only the entries the reader consumes, so the clean publication declares NO erasure.
EPUB_PARTS = {
    "mimetype": EPUB_MIMETYPE,
    "META-INF/container.xml": EPUB_CONTAINER,
    "OEBPS/content.opf": EPUB_PACKAGE,
    # Stored in the OPPOSITE order to the spine, which is the whole point of this fixture.
    "OEBPS/aa-second.xhtml": EPUB_CHAPTER_TWO,
    "OEBPS/zz-first.xhtml": EPUB_CHAPTER_ONE,
    "OEBPS/notes.xhtml": EPUB_NOTES,
}


# The publication that declares erasures, every kind. A second rendition, an unread image, an
# unread `.ncx`, a non-XHTML spine item, and — inside the spine's own XHTML — a script, a style
# sheet, a navigation document with a `page-list`, an inline SVG title and a MathML annotation.
EPUB_CONTAINER_WITH_RENDITIONS = f"""<?xml version="1.0" encoding="UTF-8"?>
<container xmlns="{OCF_NS}" version="1.0">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
    <rootfile full-path="OEBPS/rendition2.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>
"""

EPUB_PACKAGE_WITH_UNREAD = f"""<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="{OPF_NS}" version="3.0" unique-identifier="pub-id">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:identifier id="pub-id">urn:uuid:0d3f0f4a-0000-4000-8000-000000000002</dc:identifier>
    <dc:title>Erasures</dc:title>
    <dc:language>en</dc:language>
  </metadata>
  <manifest>
    <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
    <item id="body" href="text/body.xhtml" media-type="application/xhtml+xml"/>
    <item id="cover" href="images/cover.svg" media-type="image/svg+xml"/>
    <item id="pic" href="images/pic.png" media-type="image/png"/>
    <item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/>
    <item id="css" href="style/book.css" media-type="text/css"/>
  </manifest>
  <spine toc="ncx">
    <itemref idref="nav"/>
    <itemref idref="body"/>
    <itemref idref="cover"/>
  </spine>
</package>
"""

# The navigation document: a table of contents AND a page-list naming the pages of a print
# edition. The page-list is the construct docs/history/14-V2-SCOPE.md §3 forbids minting pages from, and
# it is here so the test that `pages` is empty is not vacuous.
EPUB_NAV = f"""<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="{XHTML_NS}" xmlns:epub="{EPUB_NS}">
  <head><title>Navigation</title></head>
  <body>
    <nav epub:type="toc"><ol><li><a href="text/body.xhtml">TOC-ENTRY-TEXT</a></li></ol></nav>
    <nav epub:type="page-list">
      <ol>
        <li><a href="text/body.xhtml#p17">PRINT-PAGE-17</a></li>
        <li><a href="text/body.xhtml#p18">PRINT-PAGE-18</a></li>
      </ol>
    </nav>
  </body>
</html>
"""

# `href="text/body.xhtml"` in `OEBPS/content.opf` is the entry `OEBPS/text/body.xhtml`, which is
# what makes the relative-resolution rule testable.
EPUB_BODY_WITH_UNREAD = f"""<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="{XHTML_NS}" xmlns:svg="{SVG_NS}" xmlns:m="http://www.w3.org/1998/Math/MathML">
  <head><title>HEAD-TITLE-TEXT</title></head>
  <body>
    <p>Kept sentence.</p>
    <script>var SCRIPT_SOURCE = 1;</script>
    <style>.c {{ content: "STYLE-SHEET-RULE"; }}</style>
    <p>Figure:<svg:svg><svg:title>SVG-TITLE-TEXT</svg:title></svg:svg></p>
    <p>Formula:<m:math><m:annotation>MATHML-ANNOTATION</m:annotation></m:math></p>
    <p><ruby>kanji<rt>RUBY-GUIDE</rt></ruby></p>
    <template><p>TEMPLATE-CONTENT</p></template>
    <p>Last kept sentence.</p>
  </body>
</html>
"""

EPUB_NCX = """<?xml version="1.0" encoding="UTF-8"?>
<ncx xmlns="http://www.daisy.org/z3986/2005/ncx/" version="2005-1">
  <navMap><navPoint id="n1"><navLabel><text>NCX-LABEL-TEXT</text></navLabel><content src="text/body.xhtml"/></navPoint></navMap>
</ncx>
"""

EPUB_CSS = ".c { color: black; }\n"

EPUB_COVER_SVG = f"""<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="{SVG_NS}" viewBox="0 0 10 10"><title>COVER-SVG-TITLE</title></svg>
"""

EPUB_SECOND_RENDITION = f"""<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="{OPF_NS}" version="3.0" unique-identifier="pub-id">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:identifier id="pub-id">urn:uuid:0d3f0f4a-0000-4000-8000-000000000003</dc:identifier>
    <dc:title>SECOND-RENDITION-TITLE</dc:title>
    <dc:language>en</dc:language>
  </metadata>
  <manifest><item id="b" href="text/body.xhtml" media-type="application/xhtml+xml"/></manifest>
  <spine><itemref idref="b"/></spine>
</package>
"""

# A 1x1 PNG, authored here rather than sampled: the point is that a package entry holding
# something this reader cannot read is COUNTED, and the smallest valid file makes that point.
EPUB_PICTURE = ODT_PICTURE

EPUB_WITH_UNREAD_PARTS = {
    "mimetype": EPUB_MIMETYPE,
    "META-INF/container.xml": EPUB_CONTAINER_WITH_RENDITIONS,
    "OEBPS/content.opf": EPUB_PACKAGE_WITH_UNREAD,
    "OEBPS/rendition2.opf": EPUB_SECOND_RENDITION,
    "OEBPS/nav.xhtml": EPUB_NAV,
    "OEBPS/text/body.xhtml": EPUB_BODY_WITH_UNREAD,
    "OEBPS/images/cover.svg": EPUB_COVER_SVG,
    "OEBPS/images/pic.png": EPUB_PICTURE,
    "OEBPS/toc.ncx": EPUB_NCX,
    "OEBPS/style/book.css": EPUB_CSS,
}

if __name__ == "__main__":
    write(HERE / "simple-paragraphs" / "document.docx", PARTS)
    write(HERE / "unread-parts" / "document.docx", WITH_UNREAD_PARTS)
    write(HERE / "workbook-cells" / "workbook.xlsx", WORKBOOK_PARTS)
    write(HERE / "workbook-unread-parts" / "workbook.xlsx", WORKBOOK_WITH_UNREAD_PARTS)
    write(HERE / "deck-slides" / "deck.pptx", DECK_PARTS)
    write(HERE / "deck-unread-parts" / "deck.pptx", DECK_WITH_UNREAD_PARTS)
    write(HERE / "text-paragraphs" / "document.odt", ODT_PARTS, stored_first="mimetype")
    write(
        HERE / "text-unread-parts" / "document.odt",
        ODT_WITH_UNREAD_PARTS,
        stored_first="mimetype",
    )
    write(HERE / "sheet-cells" / "workbook.ods", ODS_PARTS, stored_first="mimetype")
    write(
        HERE / "sheet-unread-parts" / "workbook.ods",
        ODS_WITH_UNREAD_PARTS,
        stored_first="mimetype",
    )
    write(
        HERE / "presentation-pages" / "presentation.odp",
        ODP_PARTS,
        stored_first="mimetype",
    )
    write(
        HERE / "presentation-unread-parts" / "presentation.odp",
        ODP_WITH_UNREAD_PARTS,
        stored_first="mimetype",
    )
    write_stream(HERE / "rich-text-paragraphs" / "document.rtf", RTF_PLAIN)
    write_stream(HERE / "rich-text-unread-destinations" / "document.rtf", RTF_UNREAD)
    write(HERE / "book-spine" / "book.epub", EPUB_PARTS, stored_first="mimetype")
    write(
        HERE / "book-unread-parts" / "book.epub",
        EPUB_WITH_UNREAD_PARTS,
        stored_first="mimetype",
    )
