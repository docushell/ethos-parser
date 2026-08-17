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


def write(path: pathlib.Path, parts: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as archive:
        for name, body in parts.items():
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


if __name__ == "__main__":
    write(HERE / "simple-paragraphs" / "document.docx", PARTS)
    write(HERE / "unread-parts" / "document.docx", WITH_UNREAD_PARTS)
    write(HERE / "workbook-cells" / "workbook.xlsx", WORKBOOK_PARTS)
    write(HERE / "workbook-unread-parts" / "workbook.xlsx", WORKBOOK_WITH_UNREAD_PARTS)
