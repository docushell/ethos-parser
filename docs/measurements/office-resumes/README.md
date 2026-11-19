# Made-up Word resumes: what `markdown --source` keeps of a DOCX resume

**Measured 2026-11-14** with the snapshot binary `target/proto/measure/bin/c9b364c/ethos-parser`
(`ethos-parser 0.66.0-dev.1`, built at c9b364c; its sha256 is in [`results.json`](results.json)).
Twenty made-up DOCX resumes, written by [`make_resumes.py`](make_resumes.py) together with an
answer key for each, and scored by [`score.py`](score.py), which [`../financial-en/`](../financial-en/)
uses too.

**Everything is invented, and all of it is CC0.** The people, companies, colleges, `example.com`
addresses and patterned phone numbers are made up, and no real resume was opened to build them. The
generator, the scorer, the keys and the documents are dedicated to the public domain under CC0 1.0,
like the engine fixtures. The documents are not committed because they regenerate byte for byte.

---

## 1. What it holds

Each resume is a DOCX built from the shapes real Word resumes use. The key records what a reader of
the Markdown must find for each shape.

| Shape in the DOCX | Key entry | Count |
| --- | --- | ---: |
| Label row with a **tab-only run**: `Designation`, `<w:r><w:tab/></w:r>`, `: X` | `lines` (`tab-run`) | 60 |
| Label row with an **in-run tab**: `Company Name<w:tab/>: Y` in one run | `lines` (`tab-in-run`) | 30 |
| **`<w:br/>` in its own run** between two label lines | `lines` (`br-run`) | 58 |
| **`<w:br/>` inside a run**: `Email : x<w:br/>Mobile : y` | `lines` (`br-in-run`) | 67 |
| Job line with a bold title and a right-tab date: `Title — Company, City<tab>Jan 2021 – Present` | `lines` (`job-tab-dates`) | 15 |
| Name in the `Title` style (on half the resumes) | `lines` (`title`) | 10 |
| A sentence with **one bold phrase** in the middle | `one_line` | 20 |
| `Heading1` and `Heading2` paragraph styles (`styles.xml`, `outlineLvl`) | `headings` | 150 |
| A numbered list (`numPr`, decimal) and bullet lists | `list_items` | 190 |
| A `w:tbl` education table | `table_rows` | 60 |
| Page header with name, e-mail and phone; a footer; one footnote | `present` | 20 each |
| Name, e-mail and phone | `present` | 20 each |
| Jobs written four ways: Heading2 line + dates below, right-tab dates, three label rows with tabs (`Duration : March 2015 to Till Now`), and three label rows joined by `<w:br/>` | `groups.jobs` | 50 |

A strict and a normalised form are recorded for every expected line: `Company Name\t: Acme` and
`Company Name : Acme`. Each key also carries the resume's fields (name, e-mail, phone, each job's
title, company, city, dates as written, start and end months and whether it is current, and
education). These fields are for a future field extractor; today they are scored only through the
checks above.

## 2. Run it

```bash
python3 docs/measurements/office-resumes/make_resumes.py      # -> target/measure-sets/office-resumes/
ETHOS_PARSER_BIN=target/proto/measure/bin/c9b364c/ethos-parser \
  python3 docs/measurements/office-resumes/score.py \
    --set target/measure-sets/office-resumes \
    --out docs/measurements/office-resumes/results.json
```

It needs Python 3.9 or later and the standard library only, with no network. `target/` is
git-ignored. **Running the generator twice gives byte-identical DOCX files and keys** (checked
2026-11-14), because zip dates are fixed, entries are sorted and the seed is 20261114. The scorer
runs `$BIN markdown --source <file>` once per document and writes the projection beside it as
`<file>.md`. It prints pass rates and a JSON line, and `--out` writes the full record: input
sha256s, generator version, binary version and sha256, per-document counts, and every failed item.

## 3. Baseline at c9b364c

| check | passed | rate |
| --- | ---: | ---: |
| `lines_strict` (tab kept) | 68/240 | 28% |
| `lines_normalised` | 98/240 | 41% |
| · `tab-run` | 0/60 | 0% |
| · `tab-in-run` | 30/30 | 100% |
| · `br-run` | 58/58 | 100% |
| · `br-in-run` | 0/67 | 0% |
| · `job-tab-dates` | 0/15 | 0% |
| · `title` | 10/10 | 100% |
| `one_line` (bold phrase mid-sentence) | 0/20 | 0% |
| `headings_line` | 150/150 | 100% |
| `headings_level` | 0/150 | 0% |
| `list_items_line` | 190/190 | 100% |
| `list_items_marked` | 0/190 | 0% |
| `table_rows` | 0/60 | 0% |
| `table_rows_exact` | 0/60 | 0% |
| `table_cells_text` | 240/240 | 100% |
| `header` · `footer` · `footnote` | 0/20 · 0/20 · 0/20 | 0% |
| `name` · `email` · `phone` | 10/20 · 20/20 · 20/20 | 50% · 100% · 100% |
| `jobs` | 16/50 | 32% |

Jobs by how they are written: Heading2 + dates below 10/10, right-tab dates 0/15, label rows with
tabs 0/10, label rows joined by an in-run `<w:br/>` 0/9, and label rows with `<w:br/>` in its own
run 6/6.

## 4. What fails today, and which plan item should move it

- **Q3, DOCX paragraphs.** Every run projects as its own paragraph. So a tab-only run separates a
  label from its value (`tab-run` 0/60), a bold phrase cuts its sentence into three paragraphs
  (`one_line` 0/20), and a right-tab job line breaks apart (`job-tab-dates` 0/15). A `<w:br/>`
  inside a run turns into a space, which joins the lines it should separate (`br-in-run` 0/67).
- **`br-run` passes only by accident.** A break in its own run ends a paragraph today, so its lines
  come out whole (58/58). If a Q3 fix joins runs into paragraphs, it must turn `<w:br/>` into a
  line break, or this column falls to zero.
- **The strict tab form cannot pass at present.** `normalize` in `markdown.rs` collapses every
  whitespace run, tabs included. Whether the Markdown should keep a tab is a question for the
  contract; the normalised column is the one Q3 should move.
- **M2, DOCX structure.** Heading styles project as plain lines (`headings_level` 0/150). Lists
  lose their numbers and bullets (`list_items_marked` 0/190). The education table comes out as one
  paragraph per cell (`table_rows` 0/60, although all 240 cells are present). The page header,
  footer and footnote are not projected at all (0/20 each). Losing the header also loses the name
  of the 10 resumes that carry it only there (`name` 10/20).

## 5. Limits

- **A synthetic set measures the shapes it was written with.** It shows which DOCX constructs the
  projection loses. It does not estimate how often real resumes use them.
- **What the scorer strips.** Before matching, it removes Markdown block markers, emphasis and
  escapes, and it reads `<br>` as a line break. So that this cannot hide a mismatch, no key string
  may contain `*`, `_`, `\` or a backtick; the generator asserts this.
- **`table_cells_text` is weak by design.** It counts a cell as kept when the cell is a whole line
  anywhere. It separates "text lost" from "structure lost" and says nothing about structure.
