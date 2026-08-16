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


if __name__ == "__main__":
    write(HERE / "simple-paragraphs" / "document.docx", PARTS)
    write(HERE / "unread-parts" / "document.docx", WITH_UNREAD_PARTS)
