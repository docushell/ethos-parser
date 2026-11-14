# Made-up English financial documents: GST invoices and bank statements in DOCX, XLSX and PDF

**Measured 2026-11-14** with the snapshot binary `target/proto/measure/bin/c9b364c/ethos-parser`
(`ethos-parser 0.66.0-dev.1`, built at c9b364c; its sha256 is in [`results.json`](results.json)).
Fifteen made-up documents are written by [`make_financial.py`](make_financial.py), each with an
answer key. They are scored by the shared scorer
[`../office-resumes/score.py`](../office-resumes/score.py).

**Everything is invented, and all of it is CC0.**
- Sellers, buyers, banks, account holders, addresses, invoice numbers and masked account numbers
  are made up.
- The GSTINs have the right 15-character shape, but each check character is deliberately one off
  the valid value, so none of them can be a registered number.
- No real invoice or statement was opened to build them.
- The generator, the keys and the documents are dedicated to the public domain under CC0 1.0, like
  the engine fixtures.

The documents are not committed; see §2 for how they reproduce.

---

## 1. What it holds

The generator builds one model per document and renders it in one of three formats:

| | GST tax invoices | Bank statements |
| --- | --- | --- |
| DOCX | 3 | 2 |
| XLSX | 2 | 2 |
| PDF (Chrome) | 3 | 3 |

- **Invoices** carry:
  - a seller block and a buyer block, each with a GSTIN;
  - invoice number, invoice date and place of supply;
  - 3–5 line items: Sl., description, HSN/SAC, qty, unit, rate, amount;
  - CGST + SGST at half the rate (same state) or IGST (another state);
  - Taxable Value and Grand Total;
  - a declaration sentence;
  - an HSN summary (the second sheet in XLSX).
- **Statements** carry:
  - bank, branch and IFSC;
  - account holder, masked account number and statement period;
  - 10–15 transactions: date, narration, reference, debit, credit, running balance;
  - an account summary with opening and closing balances.
- **Amounts** are computed in integer paise and grouped the Indian way (`4,93,098.50`,
  `12,34,567.89`). `₹` sits beside the totals and the balances.

How each format holds these shapes:

- **DOCX** (stdlib `zipfile`, fixed dates):
  - label rows `GSTIN<tab>: …` with a tab-only run, alternating with an in-run tab;
  - address blocks joined by `<w:br/>`, in its own run or inside a run;
  - line items in a `w:tbl`;
  - totals either as right-tab paragraphs or as a two-column table;
  - a page header and a footer;
  - `₹` written three ways, rotated across the set: in the amount's run (3), in a run of its own
    (3), and followed by a space (2).
- **XLSX** (stdlib `zipfile`, fixed dates):
  - two sheets with shared strings;
  - real date cells under the `dd-mm-yyyy` format;
  - amounts stored as numbers under the Indian-grouping format
    `[>=10000000]##\,##\,##\,##0.00;[>=100000]##\,##\,##0.00;##,##0.00`;
  - the same format with a `"₹"` prefix on the ₹ totals;
  - the title cell merged across the table.
- **PDF**: base-14 fonts cannot draw `₹`, so the generator writes HTML and prints it with headless
  Google Chrome from local files, with no network. Chrome writes tagged PDFs.
  - Line-item tables are ruled on some documents and unruled on others.
  - Totals sit in a borderless two-column table.
  - `₹` appears as plain text, followed by `&nbsp;`, or inside a `<span>` set in another font.
  - The HTML stays beside each PDF.

**The keys** hold:
- GSTINs, invoice and account numbers, and every date, each verbatim;
- every amount string exactly (`1,16,000.00`, which must not appear inside a longer number);
- each line item and each transaction as one table row;
- each total as its label and amount on one line;
- the ₹ adjacency rule: `₹1,16,000.00` or `₹ 1,16,000.00` on one line, never split by a line
  break or a block;
- the label lines, address lines, headings, header, footer and the declaration paragraph.

## 2. Run it

```bash
python3 docs/measurements/financial-en/make_financial.py      # -> target/measure-sets/financial-en/
ETHOS_PARSER_BIN=target/proto/measure/bin/c9b364c/ethos-parser \
  python3 docs/measurements/office-resumes/score.py \
    --set target/measure-sets/financial-en \
    --out docs/measurements/financial-en/results.json
```

- **Requirements.** Python 3.9 or later, standard library only, and Google Chrome (default
  `/Applications/Google Chrome.app/...`; pass `--chrome PATH` to use another).
- **How Chrome is run.** It gets a throwaway profile and has background networking, component
  updates, sync and extensions turned off.
- **Chrome's exit status is not used.** Chrome 157 often exits 2 from its teardown watchdog after it
  has written the PDF. The generator therefore checks the PDF itself and fails by name if the file
  is incomplete.
- **Determinism.**
  - DOCX, XLSX, HTML and keys are byte-identical across runs (fixed zip dates, sorted entries, seed
    20261115).
  - Chrome's only varying bytes are `/CreationDate` and `/ModDate`, and the generator pins them.
  - So on **Google Chrome 157.0.8103.51** the PDFs were also byte-identical across two runs
    (checked 2026-11-14).
  - Another Chrome version may lay them out differently. `results.json` records the producer and
    each input's sha256.

## 3. Baseline at c9b364c

| check | all | DOCX | PDF | XLSX |
| --- | ---: | ---: | ---: | ---: |
| `gstin` | 16/16 | 6/6 | 6/6 | 4/4 |
| `invoice_no` · `account` | 8/8 · 7/7 | 3/3 · 2/2 | 3/3 · 3/3 | 2/2 · 2/2 |
| `amounts` (verbatim, lakh grouping) | 232/311 | 109/109 | 120/120 | **3/82** |
| `dates` | 53/73 | 20/20 | 33/33 | **0/20** |
| `rupee_adjacent` | 15/26 | 5/8 | 10/11 | 0/7 |
| `table_rows` (one row, cells in order) | 63/228 | 0/69 | 63/85 | 0/74 |
| `table_rows_exact` (cells in place) | 40/228 | 0/69 | 40/85 | 0/74 |
| `table_cells_text` | 885/1008 | 335/336 | 379/380 | 171/292 |
| `totals` (label and amount on one line) | 0/56 | 0/19 | 0/22 | 0/15 |
| `lines_strict` | 33/96 | 9/45 | 24/51 | — |
| `lines_normalised` | 41/96 | 17/45 | 24/51 | — |
| · `tab-run` · `tab-in-run` | 0/13 · 8/8 | 0/13 · 8/8 | — | — |
| · `br-run` · `br-in-run` | 9/9 · 0/15 | 9/9 · 0/15 | — | — |
| · `html-label` · `html-br` | 24/24 · 0/27 | — | 24/24 · 0/27 | — |
| `one_line` (declaration) | 11/11 | 5/5 | 6/6 | — |
| `headings_line` · `headings_level` | 28/28 · 0/28 | 13/13 · 0/13 | 15/15 · 0/15 | — |
| `header` · `footer` | 0/5 · 0/5 | 0/5 · 0/5 | — | — |
| `text` (single-cell XLSX rows) | 32/32 | — | — | 32/32 |

## 4. What fails today, and which plan item should move it

- **M3, XLSX tables and dates.**
  - Each cell projects as its own paragraph (`table_rows` 0/74, `totals` 0/15).
  - Numbers come out as their stored values, `493098.50` where Excel shows `4,93,098.50`
    (`amounts` 3/82; the three that pass are under 1,000).
  - Date cells come out as serial numbers (`dates` 0/20).
  - The `₹` that the number format adds is lost (`rupee_adjacent` 0/7).
- **Lakh grouping itself survives** wherever the text is written out: in DOCX and PDF every amount
  is verbatim (109/109 and 120/120). The lakh problem is XLSX number formats.
- **₹ adjacency.**
  - DOCX: a `₹` in its own run becomes its own paragraph, which fails all 3 of that shape.
  - PDF: one of the three `₹` set in a separate font span splits from its amount.
  - The two-column totals tables also put every label and amount on separate lines (`totals`
    0/56).
- **Q3 and M2, DOCX.**
  - A tab-only run splits label from value (0/13).
  - An in-run `<w:br/>` joins the address lines (0/15).
  - `w:tbl` line items are one paragraph per cell (0/69).
  - Heading styles, the page header and the footer are lost.
- **Not in the plan items, found here.**
  - **Unruled PDF tables lose column identity.** On the two unruled statements, all 23 transaction
    rows put their amounts in the wrong column: a debit row's balance lands under Credit, and a
    credit lands under Debit, because empty cells collapse leftwards. The in-order check passes
    these rows; `table_rows_exact` catches them.
  - **Tables without empty cells read correctly.** The ruled statement reads exactly (11/11), and so
    do the unruled invoice's line items (4/4).
  - **Chrome's tagged `H1`/`H2` project as bold, not as `#`** (`headings_level` 0/15).
  - **An HTML `<br>` address block reflows into one line** (`html-br` 0/27).

## 5. Limits and doubts

- **`html-br` may be too strict for PDF.** A reflowed address is ordinary paragraph projection, and
  this check counts it as a miss because the lines hold separate fields (name, street, city). Read
  that column as a field-separation measure, not as a defect by itself.
- **XLSX expectations are what Excel displays**, including the conditional lakh format. An engine
  could reasonably refuse to evaluate number formats; the key still says what a reader sees.
- **A synthetic set measures the shapes it was written with.** It does not estimate how often real
  documents use them.
