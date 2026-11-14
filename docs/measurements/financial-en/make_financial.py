#!/usr/bin/env python3
"""Made-up English financial documents, GST tax invoices and bank statements, with answer keys.

Everything here is invented: sellers, buyers, banks, account holders, addresses, GSTINs (the
15-character shape, with a check character deliberately wrong so none can be a real one), invoice
and account numbers. This script and every file it writes are dedicated to the public domain under
CC0 1.0, like the engine fixtures.

Fifteen documents from one model each, rendered three ways:
  DOCX  stdlib zipfile, fixed dates: label rows with tabs, `<w:br/>` blocks, `w:tbl` line items, and
        `₹` written in the amount's run, in a run of its own, and with a space;
  XLSX  stdlib zipfile, fixed dates: two sheets, real date cells (`dd-mm-yyyy`), and amounts as
        numbers under an Indian-grouping number format (with a `₹` prefix on the totals);
  PDF   headless Google Chrome printing generated HTML (local files only), because a base-14 PDF
        cannot draw `₹`. The two timestamps Chrome writes are pinned, so one Chrome version gives
        the same bytes twice; another version may not. The HTML is kept beside each PDF.
Amounts use lakh grouping (`1,16,000.00`, `12,34,567.89`), computed in integer paise.

Run:  python3 docs/measurements/financial-en/make_financial.py [--out DIR] [--chrome PATH]
      (default DIR: target/measure-sets/financial-en, which git ignores)
Score with ../office-resumes/score.py (see README.md).
"""
import argparse
import datetime
import html
import itertools
import json
import pathlib
import random
import re
import string
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from xml.sax.saxutils import escape

HERE = pathlib.Path(__file__).resolve().parent
sys.dont_write_bytecode = True  # no __pycache__ beside the shared module in a tracked folder
sys.path.insert(0, str(HERE.parent / "office-resumes"))
from make_resumes import (FIRST, LAST, REL, XML, Doc, add_lines, check_key, docx_parts,  # noqa: E402
                          new_key, para, write_package)

GENERATOR = "financial-en/1"
DEFAULT_OUT = HERE.parents[2] / "target" / "measure-sets" / "financial-en"
CHROME = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
SEED = 20261115
INVOICE_FORMATS = ["docx", "pdf", "xlsx", "docx", "pdf", "xlsx", "docx", "pdf"]
STATEMENT_FORMATS = ["docx", "pdf", "xlsx", "docx", "pdf", "xlsx", "pdf"]


@dataclass(frozen=True)
class Amt:
    """An amount in paise. `rupee` means the document writes `₹` beside it."""
    paise: int
    rupee: bool = False


def inr(paise):
    """Indian digit grouping: 1,16,000.00 and 12,34,567.89."""
    r, p = divmod(paise, 100)
    head, tail = str(r)[:-3], str(r)[-3:]
    groups = []
    while head:
        groups.insert(0, head[-2:])
        head = head[:-2]
    return ",".join(groups + [tail]) + f".{p:02d}"


def show(c):
    """What a reader sees for a cell value, without any `₹`."""
    if isinstance(c, Amt):
        return inr(c.paise)
    if isinstance(c, datetime.date):
        return c.strftime("%d-%m-%Y")
    return str(c)


GSTIN_CHARS = string.digits + string.ascii_uppercase


def fake_gstin(rng, state_code, name):
    """State code, a PAN-shaped ten characters, entity digit, `Z`, and a check character that is
    one off the valid one, so the number is the right shape and can never be registered."""
    body = (state_code + "".join(rng.choice("ABCDEFGHJKLMNPQRSTUVWXYZ") for _ in range(3)) + "C" + name[0].upper()
            + f"{rng.randint(1000, 9999)}" + rng.choice("ABCDEFGHJKLMNPQRSTUVWXYZ") + "1Z")
    total = 0
    for i, ch in enumerate(body):
        v = GSTIN_CHARS.index(ch) * (2 if i % 2 else 1)
        total += v // 36 + v % 36
    valid = (36 - total % 36) % 36
    return body + GSTIN_CHARS[(valid + 1) % 36]


PLACES = [("Pune", "Maharashtra", "27", "411001"), ("Bengaluru", "Karnataka", "29", "560001"),
          ("Chennai", "Tamil Nadu", "33", "600002"), ("Ahmedabad", "Gujarat", "24", "380009"),
          ("Hyderabad", "Telangana", "36", "500003"), ("Kochi", "Kerala", "32", "682011"),
          ("Jaipur", "Rajasthan", "08", "302001"), ("Indore", "Madhya Pradesh", "23", "452001")]
STREETS = ["14 Station Road", "Plot 22, MIDC Phase II", "3rd Floor, Lakeview Towers", "Shop 7, Market Yard",
           "88 Ring Road", "Unit 5, Riverside Estate", "41 Temple Street", "Survey No. 19, Industrial Area"]
SELLERS = ["Ambervale Building Supplies Pvt Ltd", "Tamarind Electricals", "Coralnest Office Solutions LLP",
           "Granitefold Traders", "Silverbirch Hardware Co.", "Marigold Paper Mart",
           "Quillmere Tech Services Pvt Ltd", "Kestrelwick Steel Agency"]
BUYERS = ["Brightmarsh Constructions Pvt Ltd", "Riverlace Health Pvt Ltd", "Pinewhistle Motors Ltd",
          "Juniper Hollow Foods Pvt Ltd", "Monsoonlight Media", "Vellora Softworks Pvt Ltd",
          "Northmere College", "Ambervale Logistics Pvt Ltd"]
# description, HSN/SAC, unit, rate range in rupees
ITEMS = [("Portland Cement 53 Grade", "2523", "Bag", 350, 420), ("TMT Steel Bar 12 mm", "7214", "Tonne", 52000, 61000),
         ("PVC Conduit Pipe 25 mm", "3917", "Length", 85, 140), ("LED Panel Light 18 W", "9405", "Nos", 450, 900),
         ("Copper Wire 2.5 sq mm", "8544", "Coil", 1800, 2600), ("Ceramic Floor Tile 600x600", "6907", "Box", 650, 1100),
         ("Wall Putty 40 kg", "3214", "Bag", 700, 950), ("Office Chair Mesh Back", "9401", "Nos", 4200, 7800),
         ("A4 Copier Paper 75 gsm", "4802", "Ream", 240, 320), ("Annual Maintenance Service", "998713", "Job", 15000, 60000)]
BANKS = [("Kestrelwick Bank Ltd", "KSTW"), ("Ambervale Co-operative Bank", "AMBV"), ("Riverlace Urban Bank", "RVLU"),
         ("Coralnest Bank Ltd", "CRNB")]
BRANCHES = ["Baner", "Koramangala", "Adyar", "Navrangpura", "Madhapur", "Kakkanad", "Malviya Nagar", "Vijay Nagar"]
PURPOSES = ["Groceries", "Pharmacy", "School fees", "Fuel", "Mobile recharge", "Dinner", "Books"]
DECLARATION = ("Declaration: we declare that this invoice shows the actual price of the goods and services "
               "described and that all particulars are true and correct.")
STATEMENT_NOTE = ("This is a computer-generated statement and needs no signature. Please report any discrepancy "
                  "to your branch within 15 days of receipt.")


def invoice(rng, n, source):
    s_place = rng.choice(PLACES)
    b_place = s_place if n % 2 else rng.choice(PLACES)  # odd: same state, CGST + SGST; even: IGST unless it lands on the same
    seller, buyer = SELLERS[n % len(SELLERS)], BUYERS[(n * 3) % len(BUYERS)]
    s_gstin, b_gstin = fake_gstin(rng, s_place[2], seller), fake_gstin(rng, b_place[2], buyer)
    inv_no = [f"INV/2026-27/{rng.randint(1, 999):04d}", f"ST-2627-{rng.randint(1, 999):04d}",
              f"GST/26-27/{rng.randint(100, 999)}"][n % 3]
    date = datetime.date(2026, rng.randint(4, 10), rng.randint(1, 28))
    rate = rng.choice([18, 18, 12, 28, 5])
    rows, hsn_rows, taxable, tax = [], [], 0, 0
    for sl, (desc, hsn, unit, lo, hi) in enumerate(rng.sample(ITEMS, rng.randint(3, 5)), 1):
        price = rng.randint(lo, hi) * 100 + rng.choice([0, 0, 50, 25])
        qty = rng.randint(2, 25) if hi > 10000 else rng.randint(10, 300)
        amount = qty * price
        line_tax = (amount * rate + 50) // 100
        rows.append([sl, desc, hsn, qty, unit, Amt(price), Amt(amount)])
        taxable, tax = taxable + amount, tax + line_tax
        hsn_rows.append((hsn, amount, line_tax))
    intra = s_place == b_place
    half = f"{rate / 2:g}%"
    if intra:  # CGST and SGST each half the rate; computed per line, as the HSN summary shows
        cgst = sum((a * rate + 100) // 200 for _, a, _ in hsn_rows)
        taxes = [(f"CGST @ {half}", cgst), (f"SGST @ {half}", cgst)]
        summary = [["HSN/SAC", "Taxable Value", f"CGST {half}", f"SGST {half}", "Total Tax"]] + [
            [h, Amt(a), Amt((a * rate + 100) // 200), Amt((a * rate + 100) // 200), Amt(2 * ((a * rate + 100) // 200))]
            for h, a, _ in hsn_rows]
    else:
        taxes = [(f"IGST @ {rate}%", tax)]
        summary = [["HSN/SAC", "Taxable Value", f"IGST {rate}%", "Total Tax"]] + [
            [h, Amt(a), Amt(t), Amt(t)] for h, a, t in hsn_rows]
    total = taxable + sum(t for _, t in taxes)
    totals = [("Taxable Value", Amt(taxable, rupee=n % 2 == 0))] + [(l, Amt(t)) for l, t in taxes] + [
        ("Grand Total", Amt(total, rupee=True))]

    def address(place, name):
        return [name, rng.choice(STREETS), f"{place[0]} {place[3]}, {place[1]}"]

    main = [("heading", "TAX INVOICE", 1), ("lines", address(s_place, seller)), ("label", "GSTIN", s_gstin),
            ("label", "Invoice No.", inv_no), ("label", "Invoice Date", date),
            ("label", "Place of Supply", f"{b_place[1]} ({b_place[2]})"),
            ("heading", "Bill To", 2), ("lines", address(b_place, buyer)), ("label", "GSTIN", b_gstin),
            ("table", [["Sl.", "Description", "HSN/SAC", "Qty", "Unit", "Rate (₹)", "Amount (₹)"]] + rows),
            ("pairs", totals), ("para", DECLARATION)]
    second = [("heading", "HSN Summary", 2), ("table", summary)]
    return {"source": source, "sheets": [("Invoice", main), ("HSN Summary", second)],
            "header": "ORIGINAL FOR RECIPIENT", "footer": f"Subject to {s_place[0]} jurisdiction. {seller}",
            "ids": {"gstin": [s_gstin, b_gstin], "invoice_no": [inv_no]},
            "fields": {"invoice_no": inv_no, "date": show(date), "seller": {"name": seller, "gstin": s_gstin},
                       "buyer": {"name": buyer, "gstin": b_gstin}, "gst_rate": rate, "intra_state": intra,
                       "items": [[show(c) for c in r] for r in rows],
                       "totals": [[l, show(a)] for l, a in totals]}}


def statement(rng, n, source):
    bank, code = BANKS[n % len(BANKS)]
    place, branch = rng.choice(PLACES), rng.choice(BRANCHES)
    holder = f"{rng.choice(FIRST)} {rng.choice(LAST)}"
    account = f"XXXXXXXX{rng.randint(1000, 9999)}"
    month = rng.randint(4, 10)
    balance = rng.randint(2_00_000, 14_00_000) * 100 + rng.randint(0, 99)
    opening, credits, debits, rows = balance, 0, 0, []
    day = 1
    employer = rng.choice(BUYERS)
    for t in range(rng.randint(10, 15)):
        day = min(day + rng.randint(0, 3), 28)
        ref = f"{rng.randint(10 ** 11, 10 ** 12 - 1)}"
        if t == 2:
            narr, cr, dr = f"NEFT/CR/{ref}/{employer}/SALARY {datetime.date(2026, month, 1):%b %Y}".upper(), rng.randint(60_000, 2_40_000) * 100, 0
        elif rng.random() < 0.25:
            narr, cr, dr = f"NEFT/CR/{ref}/{rng.choice(SELLERS)}/Refund", rng.randint(5_000, 3_50_000) * 100 + rng.randint(0, 99), 0
        else:
            who = f"{rng.choice(FIRST)} {rng.choice(LAST)[0]}"
            narr = rng.choice([f"UPI/DR/{ref}/{who}/{rng.choice(PURPOSES)}", f"ATM WDL/{branch} {place[0]}",
                               f"IMPS/P2A/{ref}/{who}/Rent", f"ACH/DR/Electricity bill/{ref[:8]}",
                               f"CHQ PAID/{rng.randint(100000, 999999)}/{rng.choice(SELLERS)}"])
            cr, dr = 0, min(rng.randint(150, 85_000) * 100 + rng.randint(0, 99), balance - 1000)
        balance += cr - dr
        credits, debits = credits + cr, debits + dr
        rows.append([datetime.date(2026, month, day), narr, ref[-6:], Amt(dr) if dr else "", Amt(cr) if cr else "",
                     Amt(balance)])
    first, last = datetime.date(2026, month, 1), datetime.date(2026, month, 28)
    pairs = [("Opening Balance", Amt(opening, rupee=True)), ("Total Credits", Amt(credits)),
             ("Total Debits", Amt(debits)), ("Closing Balance", Amt(balance, rupee=True))]
    main = [("heading", "Statement of Account", 1),
            ("lines", [bank, f"{branch} Branch, {place[0]} {place[3]}", f"IFSC: {code}0{rng.randint(100000, 999999)}"]),
            ("label", "Account Holder", holder), ("label", "Account Number", account),
            ("label", "Statement Period", f"{show(first)} to {show(last)}"),
            ("table", [["Date", "Narration", "Ref No.", "Debit", "Credit", "Balance"]] + rows)]
    second = [("heading", "Account Summary", 2), ("pairs", pairs), ("para", STATEMENT_NOTE)]
    return {"source": source, "sheets": [("Statement", main), ("Summary", second)],
            "header": f"{bank} · Statement of Account", "footer": "Page 1 of 1",
            "ids": {"account": [account]},
            "fields": {"holder": holder, "account": account, "opening": show(Amt(opening)),
                       "closing": show(Amt(balance)), "transactions": len(rows)}}


def common_key(model):
    """The checks every format shares: ids, dates and amounts verbatim, totals, `₹` adjacency."""
    key = new_key(model["source"], GENERATOR)
    key["fields"] = model["fields"]
    key["present"].update(model["ids"])
    dates, amounts, pairs, rupee = [], [], [], []
    for _, blocks in model["sheets"]:
        for b in blocks:
            values = ([b[2]] if b[0] == "label" else [c for r in b[1] for c in r] if b[0] == "table"
                      else [a for _, a in b[1]] if b[0] == "pairs" else [])
            dates += [show(v) for v in values if isinstance(v, datetime.date)]
            amounts += [show(v) for v in values if isinstance(v, Amt)]
            rupee += [show(v) for v in values if isinstance(v, Amt) and v.rupee]
            if b[0] == "pairs":
                pairs += [[label, show(a)] for label, a in b[1]]
    key["present"]["dates"] = list(dict.fromkeys(dates))
    key["present"]["amounts"] = list(dict.fromkeys(amounts))
    key["pairs"]["totals"] = pairs
    key["rupee"] = list(dict.fromkeys(rupee))
    return key


def money_runs(a, turn):
    """`₹` beside an amount the three ways Word documents hold it: one run, its own run, a space.
    `turn` counts every `₹` amount across the set, so each way occurs."""
    t = show(a)
    if not a.rupee:
        return [(t, "")]
    return [[(f"₹{t}", "")], [("₹", ""), (t, "")], [(f"₹ {t}", "")]][next(turn) % 3]


def render_docx(model, idx, turn):
    d = Doc(model["source"], GENERATOR)
    d.key.update({k: v for k, v in common_key(model).items() if k in ("present", "pairs", "rupee", "fields")})
    labels = 0
    for _, blocks in model["sheets"]:
        for b in blocks:
            if b[0] == "heading":
                d.p([(b[1], "")], style=f"Heading{b[2]}")
            elif b[0] == "lines":
                if idx % 2:
                    d.p([("\n".join(b[1]), "")], shape="br-in-run")
                else:
                    d.p([r for line in b[1] for r in (("\n", ""), (line, ""))][1:], shape="br-run")
            elif b[0] == "label":
                labels += 1
                v = show(b[2])
                if labels % 2:
                    d.p([(b[1], ""), ("\t", ""), (f": {v}", "")], shape="tab-run")
                else:
                    d.p([(f"{b[1]}\t: {v}", "")], shape="tab-in-run")
            elif b[0] == "para":
                d.p([(b[1], "")], one_line=True)
            elif b[0] == "table":
                d.table([[show(c) for c in row] for row in b[1]])
            elif b[0] == "pairs" and idx % 2 == 0:
                for label, a in b[1]:
                    d.p([(label, ""), ("\t", "")] + money_runs(a, turn), right_tab=True)
            elif b[0] == "pairs":
                d.table([[label, money_runs(a, turn)] for label, a in b[1]])
    d.present("header", model["header"])
    d.present("footer", model["footer"])
    return docx_parts("".join(d.body), para([(model["header"], "")], style="Header"),
                      para([(model["footer"], "")], style="Footer")), d.key


CSS = """@page { size: A4; margin: 16mm; }
body { font-family: Arial, Helvetica, sans-serif; font-size: 10pt; color: #000; }
h1 { font-size: 16pt; margin: 0 0 8pt; } h2 { font-size: 12pt; margin: 12pt 0 4pt; }
p { margin: 0 0 6pt; }
table { border-collapse: collapse; width: 100%; margin: 6pt 0; }
th, td { padding: 3pt 5pt; text-align: left; vertical-align: top; }
.n { text-align: right; white-space: nowrap; } .d { white-space: nowrap; }
table.ruled th, table.ruled td { border: 0.75pt solid #000; }
table.plain th { border-bottom: 0.75pt solid #000; }
table.totals { width: 50%; margin-left: 50%; }
.sym { font-family: "Arial Unicode MS", sans-serif; }"""


def cell_class(c):
    return "n" if isinstance(c, (Amt, int)) else "d" if isinstance(c, datetime.date) else ""


def render_html(model, idx, turn):
    key = common_key(model)
    out = []
    e = html.escape

    def money_html(a):
        t = show(a)
        if not a.rupee:
            return t, t
        return [(f"₹{t}", f"₹{t}"), (f"₹&nbsp;{t}", f"₹ {t}"), (f'<span class="sym">₹</span>{t}', f"₹{t}")][next(turn) % 3]

    for _, blocks in model["sheets"]:
        for b in blocks:
            if b[0] == "heading":
                out.append(f"<h{b[2]}>{e(b[1])}</h{b[2]}>")
                key["headings"].append({"text": b[1], "level": b[2]})
            elif b[0] == "lines":
                out.append("<p>" + "<br>".join(e(x) for x in b[1]) + "</p>")
                add_lines(key, "\n".join(b[1]), "html-br")
            elif b[0] == "label":
                out.append(f"<p>{e(b[1])} : {e(show(b[2]))}</p>")
                add_lines(key, f"{b[1]} : {show(b[2])}", "html-label")
            elif b[0] == "para":
                out.append(f"<p>{e(b[1])}</p>")
                key["one_line"].append(b[1])
            elif b[0] == "table":
                head, *body = b[1]
                cls = "ruled" if idx % 2 == 0 else "plain"
                out.append(f'<table class="{cls}"><thead><tr>' + "".join(f"<th>{e(h)}</th>" for h in head) + "</tr></thead><tbody>"
                           + "".join("<tr>" + "".join(f'<td class="{cell_class(c)}">{e(show(c))}</td>' for c in r) + "</tr>"
                                     for r in body)
                           + "</tbody></table>")
                key["table_rows"] += [[show(c) for c in r] for r in b[1]]
            elif b[0] == "pairs":
                cells = []
                for label, a in b[1]:
                    markup, text = money_html(a)
                    cells.append(f'<tr><td>{e(label)}</td><td class="n">{markup}</td></tr>')
                    key["table_rows"].append([label, text])
                out.append('<table class="totals">' + "".join(cells) + "</table>")
    title = model["sheets"][0][1][0][1]
    return (f'<!doctype html>\n<html lang="en"><head><meta charset="utf-8"><title>{e(title)}</title>'
            f"<style>{CSS}</style></head><body>\n" + "\n".join(out) + "\n</body></html>\n"), key


def print_pdf(chrome, html_path, pdf_path):
    """Chrome prints, then often exits 2 from its teardown watchdog; the PDF is judged, not the code."""
    pdf_path.unlink(missing_ok=True)
    with tempfile.TemporaryDirectory() as profile:
        subprocess.run([chrome, "--headless", "--disable-gpu", "--no-pdf-header-footer", f"--user-data-dir={profile}",
                        "--no-first-run", "--no-default-browser-check", "--disable-background-networking",
                        "--disable-component-update", "--disable-sync", "--disable-extensions",
                        f"--print-to-pdf={pdf_path}", html_path.resolve().as_uri()],
                       capture_output=True, timeout=120)
    data = pdf_path.read_bytes() if pdf_path.exists() else b""
    if not data.startswith(b"%PDF-") or b"%%EOF" not in data[-64:]:
        raise SystemExit(f"Chrome wrote no complete PDF for {html_path.name}")
    pdf_path.write_bytes(re.sub(rb"/(CreationDate|ModDate) \(D:\d{14}", rb"/\1 (D:20260101000000", data))


XLSX_STYLES = XML + """<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
<numFmts count="3"><numFmt numFmtId="164" formatCode="dd-mm-yyyy"/>
<numFmt numFmtId="165" formatCode="[&gt;=10000000]##\\,##\\,##\\,##0.00;[&gt;=100000]##\\,##\\,##0.00;##,##0.00"/>
<numFmt numFmtId="166" formatCode="[&gt;=10000000]&quot;₹&quot;##\\,##\\,##\\,##0.00;[&gt;=100000]&quot;₹&quot;##\\,##\\,##0.00;&quot;₹&quot;##,##0.00"/></numFmts>
<fonts count="2"><font><sz val="11"/><name val="Calibri"/></font><font><b/><sz val="11"/><name val="Calibri"/></font></fonts>
<fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills>
<borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>
<cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>
<cellXfs count="5"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
<xf numFmtId="164" fontId="0" fillId="0" borderId="0" xfId="0" applyNumberFormat="1"/>
<xf numFmtId="165" fontId="0" fillId="0" borderId="0" xfId="0" applyNumberFormat="1"/>
<xf numFmtId="166" fontId="0" fillId="0" borderId="0" xfId="0" applyNumberFormat="1"/>
<xf numFmtId="0" fontId="1" fillId="0" borderId="0" xfId="0" applyFont="1"/></cellXfs>
<cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles>
</styleSheet>"""
EPOCH = datetime.date(1899, 12, 30)
S_DATE, S_AMOUNT, S_RUPEE, S_BOLD = 1, 2, 3, 4
SML = 'xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"'
CT_SML = "application/vnd.openxmlformats-officedocument.spreadsheetml"


def render_xlsx(model):
    key = common_key(model)
    strings = {}
    sheets = []
    for name, blocks in model["sheets"]:
        rows, merges, width = [], [], 2
        for b in blocks:
            if b[0] == "heading":
                rows.append([(b[1], S_BOLD)])
                merges.append(len(rows))
            elif b[0] == "lines":
                rows += [[(x, 0)] for x in b[1]]
            elif b[0] == "label":
                rows.append([(b[1], S_BOLD), (b[2], 0)])
            elif b[0] == "para":
                rows.append([(b[1], 0)])
            elif b[0] == "table":
                width = len(b[1][0])
                rows.append([(h, S_BOLD) for h in b[1][0]])
                rows += [[(c, 0) for c in r] for r in b[1][1:]]
            elif b[0] == "pairs":
                rows += [[(None, 0)] * (width - 2) + [(label, S_BOLD), (a, 0)] for label, a in b[1]]
        xml_rows = []
        for r, row in enumerate(rows, 1):
            cells, shown = [], []
            for c, (v, style) in enumerate(row):
                ref = f"{'ABCDEFGHIJ'[c]}{r}"
                if v is None or v == "":
                    shown.append("")
                    continue
                if isinstance(v, Amt):
                    cells.append(f'<c r="{ref}" s="{S_RUPEE if v.rupee else S_AMOUNT}"><v>{v.paise // 100}.{v.paise % 100:02d}</v></c>')
                    shown.append(("₹" if v.rupee else "") + show(v))
                elif isinstance(v, datetime.date):
                    cells.append(f'<c r="{ref}" s="{S_DATE}"><v>{(v - EPOCH).days}</v></c>')
                    shown.append(show(v))
                elif isinstance(v, int):
                    cells.append(f'<c r="{ref}"><v>{v}</v></c>')
                    shown.append(str(v))
                else:
                    idx = strings.setdefault(v, len(strings))
                    bold = f' s="{style}"' if style else ""
                    cells.append(f'<c r="{ref}" t="s"{bold}><v>{idx}</v></c>')
                    shown.append(v)
            xml_rows.append(f'<row r="{r}">{"".join(cells)}</row>')
            if sum(map(bool, shown)) > 1:  # a row of cells, kept in place
                key["table_rows"].append(shown)
            else:
                key["present"].setdefault("text", []).extend(x for x in shown if x)
        merge = (f'<mergeCells count="{len(merges)}">' + "".join(
            f'<mergeCell ref="A{r}:{"ABCDEFGHIJ"[width - 1]}{r}"/>' for r in merges) + "</mergeCells>") if merges else ""
        sheets.append((name, XML + f'<worksheet {SML}><sheetData>{"".join(xml_rows)}</sheetData>{merge}</worksheet>'))
    parts = {f"xl/worksheets/sheet{i}.xml": body for i, (_, body) in enumerate(sheets, 1)}
    parts["xl/sharedStrings.xml"] = XML + f'<sst {SML} count="{len(strings)}" uniqueCount="{len(strings)}">' + "".join(
        f'<si><t xml:space="preserve">{escape(s)}</t></si>' for s in strings) + "</sst>"
    parts["xl/styles.xml"] = XLSX_STYLES
    parts["xl/workbook.xml"] = XML + (
        f'<workbook {SML} xmlns:r="{REL}"><sheets>' + "".join(
            f'<sheet name="{escape(n)}" sheetId="{i}" r:id="rId{i}"/>' for i, (n, _) in enumerate(sheets, 1))
        + "</sheets></workbook>")
    n = len(sheets)
    parts["xl/_rels/workbook.xml.rels"] = XML + (
        '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">' + "".join(
            f'<Relationship Id="rId{i}" Type="{REL}/worksheet" Target="worksheets/sheet{i}.xml"/>' for i in range(1, n + 1))
        + f'<Relationship Id="rId{n + 1}" Type="{REL}/sharedStrings" Target="sharedStrings.xml"/>'
        f'<Relationship Id="rId{n + 2}" Type="{REL}/styles" Target="styles.xml"/></Relationships>')
    parts["_rels/.rels"] = XML + (
        '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
        f'<Relationship Id="rId1" Type="{REL}/officeDocument" Target="xl/workbook.xml"/></Relationships>')
    parts["[Content_Types].xml"] = XML + (
        '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
        '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
        '<Default Extension="xml" ContentType="application/xml"/>'
        f'<Override PartName="/xl/workbook.xml" ContentType="{CT_SML}.sheet.main+xml"/>'
        + "".join(f'<Override PartName="/xl/worksheets/sheet{i}.xml" ContentType="{CT_SML}.worksheet+xml"/>'
                  for i in range(1, n + 1))
        + f'<Override PartName="/xl/sharedStrings.xml" ContentType="{CT_SML}.sharedStrings+xml"/>'
        f'<Override PartName="/xl/styles.xml" ContentType="{CT_SML}.styles+xml"/></Types>')
    return parts, key


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--out", type=pathlib.Path, default=DEFAULT_OUT)
    ap.add_argument("--chrome", default=CHROME)
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    chrome_version = subprocess.run([args.chrome, "--version"], capture_output=True, text=True, check=True).stdout.strip()
    rng = random.Random(SEED)
    plan = [("invoice", f, invoice) for f in INVOICE_FORMATS] + [("statement", f, statement) for f in STATEMENT_FORMATS]
    counters, turns = {}, {"docx": itertools.count(), "pdf": itertools.count()}
    for kind, fmt, build in plan:
        n = counters[kind] = counters.get(kind, 0) + 1
        source = f"{kind}-{n:02d}.{fmt}"
        model = build(rng, n, source)
        if fmt == "docx":
            parts, key = render_docx(model, n, turns[fmt])
            write_package(args.out / source, parts)
        elif fmt == "xlsx":
            parts, key = render_xlsx(model)
            write_package(args.out / source, parts)
        else:
            page, key = render_html(model, n, turns[fmt])
            html_path = (args.out / source).with_suffix(".html")
            html_path.write_text(page, encoding="utf-8")
            print_pdf(args.chrome, html_path, args.out / source)
            key["producer"] = chrome_version
        check_key(key)
        (args.out / source).with_suffix(".key.json").write_text(
            json.dumps(key, ensure_ascii=False, indent=1, sort_keys=True) + "\n", encoding="utf-8")
    print(f"wrote {len(plan)} documents and keys to {args.out} ({chrome_version} for the PDFs)")


if __name__ == "__main__":
    main()
