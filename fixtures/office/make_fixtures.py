#!/usr/bin/env python3
# Copyright 2026 The ethos-engine maintainers
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


def write(path: pathlib.Path, parts: dict, stored_first=None) -> None:
    """Write a deterministic package.

    `stored_first` names an entry that must be written FIRST and UNCOMPRESSED. Only ODF needs it,
    and it is not a convenience: the OpenDocument package specification requires the `mimetype`
    entry to be the first file and to be stored, so that a consumer can identify the document from
    the leading bytes without inflating anything. `engine-office`'s reader checks both, so a
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
    "[Content_Types].xml": CONTENT_TYPES,
    "_rels/.rels": RELS,
    "word/document.xml": BODY_ONLY,
    "word/header1.xml": HEADER,
    "word/footer1.xml": FOOTER,
    "word/footnotes.xml": FOOTNOTES,
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
    "[Content_Types].xml": WORKBOOK_CONTENT_TYPES,
    "_rels/.rels": WORKBOOK_RELS,
    "xl/workbook.xml": ONE_SHEET_ONLY,
    "xl/_rels/workbook.xml.rels": ONE_SHEET_RELS,
    "xl/worksheets/sheet1.xml": ONE_CELL,
    "xl/charts/chart1.xml": CHART,
    "xl/drawings/drawing1.xml": DRAWING,
    "xl/comments1.xml": COMMENTS,
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
    "[Content_Types].xml": DECK_CONTENT_TYPES,
    "_rels/.rels": DECK_RELS,
    "ppt/presentation.xml": ONE_SLIDE_ONLY,
    "ppt/_rels/presentation.xml.rels": ONE_SLIDE_RELS,
    "ppt/slides/slide1.xml": SLIDE_WITH_UNREAD_SHAPES,
    "ppt/notesSlides/notesSlide1.xml": NOTES_SLIDE,
    "ppt/slideLayouts/slideLayout1.xml": SLIDE_LAYOUT,
    "ppt/slideMasters/slideMaster1.xml": SLIDE_MASTER,
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
ODT_PICTURE = bytes.fromhex(
    "89504e470d0a1a0a0000000d494844520000000100000001080600000"
    "01f15c4890000000a49444154789c6300010000050001"
    "0d0a2db40000000049454e44ae426082"
)

ODT_WITH_UNREAD_PARTS = {
    "mimetype": ODT_MIMETYPE,
    "META-INF/manifest.xml": ODT_MANIFEST_WITH_PARTS,
    "content.xml": ODT_BODY_WITH_REGIONS,
    "styles.xml": ODT_STYLES,
    "meta.xml": ODT_META,
    "Pictures/10000000.png": ODT_PICTURE,
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
