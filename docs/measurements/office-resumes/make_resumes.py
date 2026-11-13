#!/usr/bin/env python3
"""Made-up Word resumes and an answer key for each, written deterministically.

Everything here is invented: the people, the companies, the colleges, the `example.com` e-mail
addresses and the patterned phone numbers. This script and every file it writes are dedicated to
the public domain under CC0 1.0, like the engine fixtures.

Each resume is built from the shapes real Word resumes use: label/value rows with a tab-only run or
an in-run tab before the colon, `<w:br/>` both in its own run and inside a run, a sentence with one
bold phrase in the middle, Heading1/Heading2 paragraph styles, a numbered and a bulleted list
(`w:numPr`), a `w:tbl` education table, a page header carrying the name and contact line, a footer,
one footnote, and jobs written three ways ("Title — Company, City" with dates, and label rows).

The key is built in the same call that writes each paragraph, so it can only claim what the file
holds. `score.py` beside this file reads it.

Run:  python3 docs/measurements/office-resumes/make_resumes.py [--out DIR]
      (default DIR: target/measure-sets/office-resumes, which git ignores)
"""
import argparse
import json
import pathlib
import random
import re
import zipfile
from xml.sax.saxutils import escape

GENERATOR = "office-resumes/1"
ROOT = pathlib.Path(__file__).resolve().parents[3]
DEFAULT_OUT = ROOT / "target" / "measure-sets" / "office-resumes"
FIXED_DATE = (2026, 1, 1, 0, 0, 0)
SEED = 20261114
COUNT = 20

XML = '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n'
W = ('xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" '
     'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"')
REL = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
NUM_DECIMAL, NUM_BULLET = 1, 2

# --- OOXML package helpers (make_financial.py imports these) ------------------------------------


def write_package(path, parts):
    """A zip with fixed dates and sorted entries, so two runs write the same bytes."""
    path.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as z:
        for name in sorted(parts):
            info = zipfile.ZipInfo(name, date_time=FIXED_DATE)
            info.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info, parts[name])


def run(text, fmt=""):
    """One `w:r`. A tab or newline inside `text` becomes `<w:tab/>` or `<w:br/>` inside this run, so
    a run whose whole text is "\\t" or "\\n" is the tab-only or break-only run Word writes."""
    rpr = {"": "", "b": "<w:rPr><w:b/></w:rPr>", "i": "<w:rPr><w:i/></w:rPr>"}[fmt]
    inner = "".join("<w:tab/>" if s == "\t" else "<w:br/>" if s == "\n"
                    else f'<w:t xml:space="preserve">{escape(s)}</w:t>'
                    for s in re.split(r"([\t\n])", text) if s)
    return f"<w:r>{rpr}{inner}</w:r>"


def para(runs, style=None, num=None, right_tab=False, extra=""):
    """A `w:p` from (text, fmt) pairs, one run each; `extra` is raw XML after the runs."""
    ppr = (f'<w:pStyle w:val="{style}"/>' if style else "") + (
        f'<w:numPr><w:ilvl w:val="0"/><w:numId w:val="{num}"/></w:numPr>' if num else "") + (
        '<w:tabs><w:tab w:val="right" w:pos="9026"/></w:tabs>' if right_tab else "")
    return (f"<w:p>{'<w:pPr>' + ppr + '</w:pPr>' if ppr else ''}"
            f"{''.join(run(t, f) for t, f in runs)}{extra}</w:p>")


def table(rows, header=True):
    """A `w:tbl`. A cell is a string, or a list of (text, fmt) runs."""
    def cell(c, bold):
        runs = c if isinstance(c, list) else [(c, "b" if bold else "")]
        return f'<w:tc><w:tcPr><w:tcW w:w="0" w:type="auto"/></w:tcPr>{para(runs)}</w:tc>'
    grid = "".join('<w:gridCol w:w="2000"/>' for _ in rows[0])
    body = "".join("<w:tr>" + "".join(cell(c, header and i == 0) for c in row) + "</w:tr>"
                   for i, row in enumerate(rows))
    return (f'<w:tbl><w:tblPr><w:tblStyle w:val="TableGrid"/><w:tblW w:w="0" w:type="auto"/>'
            f'</w:tblPr><w:tblGrid>{grid}</w:tblGrid>{body}</w:tbl>')


STYLES = XML + f"""<w:styles {W}>
<w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/><w:rPr><w:sz w:val="21"/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="Title"><w:name w:val="Title"/><w:basedOn w:val="Normal"/><w:rPr><w:b/><w:sz w:val="44"/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:pPr><w:keepNext/><w:outlineLvl w:val="0"/></w:pPr><w:rPr><w:b/><w:sz w:val="30"/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="Heading2"><w:name w:val="heading 2"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:pPr><w:keepNext/><w:outlineLvl w:val="1"/></w:pPr><w:rPr><w:b/><w:sz w:val="24"/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="Header"><w:name w:val="header"/><w:basedOn w:val="Normal"/></w:style>
<w:style w:type="paragraph" w:styleId="Footer"><w:name w:val="footer"/><w:basedOn w:val="Normal"/></w:style>
<w:style w:type="paragraph" w:styleId="FootnoteText"><w:name w:val="footnote text"/><w:basedOn w:val="Normal"/><w:rPr><w:sz w:val="18"/></w:rPr></w:style>
<w:style w:type="character" w:styleId="FootnoteReference"><w:name w:val="footnote reference"/><w:rPr><w:vertAlign w:val="superscript"/></w:rPr></w:style>
<w:style w:type="table" w:styleId="TableGrid"><w:name w:val="Table Grid"/><w:tblPr><w:tblBorders><w:top w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:left w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:bottom w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:right w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:insideH w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:insideV w:val="single" w:sz="4" w:space="0" w:color="auto"/></w:tblBorders></w:tblPr></w:style>
</w:styles>"""

NUMBERING = XML + f"""<w:numbering {W}>
<w:abstractNum w:abstractNumId="0"><w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="decimal"/><w:lvlText w:val="%1."/><w:lvlJc w:val="left"/><w:pPr><w:ind w:left="720" w:hanging="360"/></w:pPr></w:lvl></w:abstractNum>
<w:abstractNum w:abstractNumId="1"><w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="bullet"/><w:lvlText w:val="•"/><w:lvlJc w:val="left"/><w:pPr><w:ind w:left="720" w:hanging="360"/></w:pPr></w:lvl></w:abstractNum>
<w:num w:numId="{NUM_DECIMAL}"><w:abstractNumId w:val="0"/></w:num>
<w:num w:numId="{NUM_BULLET}"><w:abstractNumId w:val="1"/></w:num>
</w:numbering>"""

FOOTNOTE_REF = ('<w:r><w:rPr><w:rStyle w:val="FootnoteReference"/></w:rPr>'
                '<w:footnoteReference w:id="1"/></w:r>')
CT_MAIN = "application/vnd.openxmlformats-officedocument.wordprocessingml"


def docx_parts(body, header=None, footer=None, footnote=None):
    """The parts of a DOCX package: body XML plus optional header, footer and one footnote."""
    parts, rels, overrides = {}, [("rIdS", "styles", "styles.xml"), ("rIdN", "numbering", "numbering.xml")], []
    sect = ""
    if header:
        parts["word/header1.xml"] = XML + f"<w:hdr {W}>{header}</w:hdr>"
        rels.append(("rIdH", "header", "header1.xml"))
        overrides.append(("/word/header1.xml", "header"))
        sect += '<w:headerReference w:type="default" r:id="rIdH"/>'
    if footer:
        parts["word/footer1.xml"] = XML + f"<w:ftr {W}>{footer}</w:ftr>"
        rels.append(("rIdF", "footer", "footer1.xml"))
        overrides.append(("/word/footer1.xml", "footer"))
        sect += '<w:footerReference w:type="default" r:id="rIdF"/>'
    if footnote:
        parts["word/footnotes.xml"] = XML + (
            f'<w:footnotes {W}><w:footnote w:type="separator" w:id="-1"><w:p><w:r><w:separator/></w:r></w:p></w:footnote>'
            '<w:footnote w:type="continuationSeparator" w:id="0"><w:p><w:r><w:continuationSeparator/></w:r></w:p></w:footnote>'
            f'<w:footnote w:id="1">{para([(footnote, "")], style="FootnoteText")}</w:footnote></w:footnotes>')
        rels.append(("rIdFN", "footnotes", "footnotes.xml"))
        overrides.append(("/word/footnotes.xml", "footnotes"))
    sect += ('<w:pgSz w:w="11906" w:h="16838"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" '
             'w:left="1440" w:header="708" w:footer="708" w:gutter="0"/>')
    parts["word/document.xml"] = XML + f"<w:document {W}><w:body>{body}<w:sectPr>{sect}</w:sectPr></w:body></w:document>"
    parts["word/styles.xml"] = STYLES
    parts["word/numbering.xml"] = NUMBERING
    overrides += [("/word/document.xml", "document.main"), ("/word/styles.xml", "styles"),
                  ("/word/numbering.xml", "numbering")]
    parts["[Content_Types].xml"] = XML + (
        '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
        '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
        '<Default Extension="xml" ContentType="application/xml"/>'
        + "".join(f'<Override PartName="{p}" ContentType="{CT_MAIN}.{k}+xml"/>' for p, k in sorted(overrides))
        + "</Types>")
    parts["_rels/.rels"] = XML + (
        '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
        f'<Relationship Id="rId1" Type="{REL}/officeDocument" Target="word/document.xml"/></Relationships>')
    parts["word/_rels/document.xml.rels"] = XML + (
        '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
        + "".join(f'<Relationship Id="{i}" Type="{REL}/{t}" Target="{g}"/>' for i, t, g in rels)
        + "</Relationships>")
    return parts


def new_key(source, generator):
    """The answer-key shape score.py reads. Every list may stay empty."""
    return {"source": source, "generator": generator, "lines": [], "one_line": [], "headings": [],
            "list_items": [], "table_rows": [], "present": {}, "groups": {}, "pairs": {}, "rupee": [],
            "fields": {}}


def add_lines(key, text, shape):
    """Each visible line of `text` must be one whole line of the Markdown: strict keeps the tab,
    normalised collapses every whitespace run to one space."""
    for line in text.split("\n"):
        key["lines"].append({"strict": line, "normalised": " ".join(line.split()), "shape": shape})


class Doc:
    """A DOCX body and its key, built together."""

    def __init__(self, source, generator=GENERATOR):
        self.body = []
        self.key = new_key(source, generator)

    def p(self, runs, shape=None, one_line=False, style=None, num=None, **kw):
        self.body.append(para(runs, style=style, num=num, **kw))
        text = "".join(t for t, _ in runs)
        if style in ("Heading1", "Heading2"):
            self.key["headings"].append({"text": text, "level": int(style[-1])})
        if num:
            self.key["list_items"].append({"text": text, "ordered": num == NUM_DECIMAL})
        if shape:
            add_lines(self.key, text, shape)
        if one_line:
            self.key["one_line"].append(text)
        return [" ".join(x.split()) for x in text.split("\n")]

    def table(self, rows):
        self.body.append(table(rows))
        self.key["table_rows"] += [[c if isinstance(c, str) else "".join(t for t, _ in c) for c in r] for r in rows]

    def present(self, group, *texts):
        self.key["present"].setdefault(group, []).extend(texts)


# --- Made-up content ------------------------------------------------------------------------------

FIRST = ["Asha", "Rohan", "Meera", "Kabir", "Nisha", "Arjun", "Priya", "Devansh", "Kavya", "Sameer",
         "Ishita", "Neel", "Tara", "Vikram", "Anika", "Farhan", "Leela", "Omkar", "Ritu", "Yash"]
LAST = ["Kulkarni", "Deshpande", "Menon", "Iyer", "Bhatt", "Rao", "Shah", "Nair", "Pillai", "Joshi",
        "Kapoor", "Sethi", "Varma", "Gokhale", "Patil", "Chawla", "Dutta", "Hegde"]
# Patterned, never sampled, and written in the formats resumes use.
PHONES = ["+91 98765 43210", "+91 91234 50607", "+91-99887-76655", "+91 90000 12345", "(+91) 98989 89898",
          "+91 97531 86420", "+91 92468 13579", "098765 01234", "+91 99001 12233", "+91 93210 98765"]
COMPANIES = ["Vellora Softworks Pvt Ltd", "Brightmarsh Analytics LLP", "Kestrelwick Retail Ltd",
             "Ambervale Logistics Pvt Ltd", "Tamarind Circuitworks", "Coralnest Finserv Ltd",
             "Juniper Hollow Foods Pvt Ltd", "Monsoonlight Media", "Granitefold Infra Ltd",
             "Riverlace Health Pvt Ltd", "Quillmere Consulting", "Pinewhistle Motors Ltd"]
CITIES = [("Pune", "411038"), ("Bengaluru", "560034"), ("Hyderabad", "500081"), ("Chennai", "600042"),
          ("Mumbai", "400076"), ("Kochi", "682024"), ("Ahmedabad", "380015"), ("Indore", "452010")]
TITLES = ["Data Analyst", "Senior Software Engineer", "Accounts Executive", "Product Manager",
          "HR Generalist", "Sales Manager", "QA Engineer", "Business Analyst", "Operations Lead",
          "Marketing Associate", "Finance Controller", "Customer Success Manager"]
DOMAINS = ["retail demand forecasting", "payments reconciliation", "warehouse operations",
           "B2B software sales", "regional payroll", "mobile app testing", "vendor onboarding"]
TRAITS = ["clean month-end closes", "calm release weeks", "dashboards people actually open",
          "short hiring cycles", "steady quarter-on-quarter growth"]
ACHIEVEMENTS = ["Cut the monthly close from nine working days to five",
                "Moved 40 vendor contracts onto one renewal calendar",
                "Built a weekly sales dashboard used by six regional heads",
                "Reduced test-cycle time by a third with a shared regression suite",
                "Trained twelve new joiners on the reconciliation workflow",
                "Led the move of payroll for 900 staff to a new system",
                "Won the regional team award two years running"]
SKILLS = ["Advanced Excel", "SQL", "Python", "Power BI", "Tally", "Jira", "Stakeholder reporting",
          "GST filing", "Selenium", "Salesforce"]
DEGREES = [("B.Tech, Computer Science", "Marigold Institute of Technology"),
           ("B.Com", "Tamarind College of Commerce"), ("MBA, Finance", "Coralnest School of Management"),
           ("M.Sc, Statistics", "Ambervale University"), ("BBA", "Northmere College"),
           ("B.E., Mechanical", "Silverbirch Engineering College")]
RESPONSIBILITY = ["Owned the weekly reporting pack for leadership", "Handled escalations from key accounts",
                  "Reconciled ledgers across three entities", "Mentored two junior colleagues",
                  "Ran quarterly planning with the regional teams"]
MON = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]
MONTH = ["January", "February", "March", "April", "May", "June", "July", "August", "September",
         "October", "November", "December"]
LANGS = ["English, Hindi, Marathi", "English, Hindi", "English, Tamil, Hindi", "English, Kannada",
         "English, Malayalam, Hindi", "English, Gujarati, Hindi"]


def jobs_for(rng, n, city):
    """n jobs, newest first; the newest is current. Months as (year, month index)."""
    start = (rng.randint(2019, 2023), rng.randint(0, 11))
    out = [dict(title=None, start=start, end=None)]
    for _ in range(n - 1):
        end = (start[0], start[1] - 1) if start[1] else (start[0] - 1, 11)
        months = rng.randint(18, 48)
        y, m = divmod(end[0] * 12 + end[1] - months, 12)
        start = (y, m)
        out.append(dict(title=None, start=start, end=end))
    titles, companies = rng.sample(TITLES, n), rng.sample(COMPANIES, n)
    for j, t, c in zip(out, titles, companies):
        j.update(title=t, company=c, city=rng.choice(CITIES)[0] if rng.random() < 0.5 else city)
    return out


def resume(i, rng):
    first, last = FIRST[i], rng.choice(LAST)
    name = f"{first} {last}"
    email = [f"{first.lower()}.{last.lower()}@example.com", f"{first.lower()}{last[0].lower()}@example.com",
             f"{first[0].lower()}.{last.lower()}{rng.randint(10, 99)}@example.com"][i % 3]
    phone = PHONES[i % len(PHONES)]
    city, pin = rng.choice(CITIES)
    d = Doc(f"resume-{i + 1:02d}.docx")
    br_in = i % 2 == 0

    if i % 2 == 0:  # half carry the name in the body as well; the rest only in the page header
        d.p([(name, "")], style="Title", shape="title")
    if br_in:
        d.p([(f"Email : {email}\nMobile : {phone}", "")], shape="br-in-run")
    else:
        d.p([(f"Email : {email}", ""), ("\n", ""), (f"Mobile : {phone}", "")], shape="br-run")

    jobs = jobs_for(rng, 2 + i % 2, city)
    d.p([("Professional Summary", "")], style="Heading1")
    d.p([(f"{jobs[0]['title']} with ", ""), (f"{rng.choice(['four', 'five', 'six', 'seven', 'nine'])} years of experience", "b"),
         (f" in {rng.choice(DOMAINS)}, known for {rng.choice(TRAITS)}.", "")], one_line=True)

    d.p([("Work Experience", "")], style="Heading1")
    style = ("heading", "tab-dates", "label-tab", "label-br")[i % 4]
    job_lines, job_fields = [], []
    for k, j in enumerate(jobs):
        s, e = j["start"], j["end"]
        short = f"{MON[s[1]]} {s[0]} – " + (f"{MON[e[1]]} {e[0]}" if e else "Present")
        long = f"{MONTH[s[1]]} {s[0]} to " + (f"{MONTH[e[1]]} {e[0]}" if e else "Till Now")
        head = f"{j['title']} — {j['company']}, {j['city']}"
        if style == "heading":
            lines = d.p([(head, "")], style="Heading2") + d.p([(short, "i")])
            dates = short
        elif style == "tab-dates":
            lines = d.p([(j["title"], "b"), (f" — {j['company']}, {j['city']}", ""), ("\t", ""), (short, "")],
                        right_tab=True, shape="job-tab-dates")
            dates = short
        elif style == "label-tab":
            lines = d.p([("Designation", ""), ("\t", ""), (f": {j['title']}", "")], shape="tab-run")
            lines += d.p([(f"Company Name\t: {j['company']}, {j['city']}", "")], shape="tab-in-run")
            lines += d.p([("Duration", ""), ("\t", ""), (f": {long}", "")], shape="tab-run")
            dates = long
        else:
            rows = [f"Designation : {j['title']}", f"Company : {j['company']}, {j['city']}", f"Duration : {long}"]
            if (i // 4) % 2 == 0:
                lines = d.p([("\n".join(rows), "")], shape="br-in-run")
            else:
                runs = [(rows[0], ""), ("\n", ""), (rows[1], ""), ("\n", ""), (rows[2], "")]
                lines = d.p(runs, shape="br-run")
            dates = long
        d.p([(rng.choice(RESPONSIBILITY), "")], num=NUM_BULLET)
        job_lines.append(lines)
        job_fields.append({"title": j["title"], "company": j["company"], "city": j["city"], "dates": dates,
                           "start": f"{s[0]}-{s[1] + 1:02d}", "end": f"{e[0]}-{e[1] + 1:02d}" if e else None,
                           "current": e is None})

    d.p([("Key Achievements", "")], style="Heading1")
    for a in rng.sample(ACHIEVEMENTS, 3):
        d.p([(a, "")], num=NUM_DECIMAL)

    d.p([("Skills", "")], style="Heading1")
    d.p([("Technical Skills", "")], style="Heading2")
    for s in rng.sample(SKILLS, 4):
        d.p([(s, "")], num=NUM_BULLET)

    d.p([("Education", "")], style="Heading1")
    year = jobs[-1]["start"][0] - rng.randint(0, 1)
    edu = []
    for (deg, inst), y in zip(rng.sample(DEGREES, 2), (year, year - rng.choice([2, 3]))):
        score = f"{rng.randint(65, 92)}.{rng.randint(0, 9)}%" if rng.random() < 0.5 else f"{rng.randint(6, 9)}.{rng.randint(0, 9)} CGPA"
        edu.append({"degree": deg, "institution": f"{inst}, {rng.choice(CITIES)[0]}", "year": str(y), "score": score})
    d.table([["Qualification", "Institution", "Year", "Score"]]
            + [[e["degree"], e["institution"], e["year"], e["score"]] for e in edu])

    d.p([("Personal Details", "")], style="Heading1")
    dob = f"{rng.randint(1, 28)} {rng.choice(MONTH)} {int(edu[1]['year']) - rng.randint(21, 22)}"
    d.p([("Date of Birth", ""), ("\t", ""), (f": {dob}", "")], shape="tab-run")
    d.p([("Nationality\t: Indian", "")], shape="tab-in-run")
    d.p([("Languages Known", ""), ("\t", ""), (f": {rng.choice(LANGS)}", "")], shape="tab-run")
    addr = [f"Address : {rng.randint(2, 98)} {rng.choice(['Lakeview', 'Hillside', 'Station', 'Temple', 'Garden'])} Road",
            f"{rng.choice(['Kothrud', 'Indiranagar', 'Madhapur', 'Adyar', 'Powai', 'Kakkanad'])}, {city} {pin}"]
    if br_in:  # the opposite break kind to the contact block, so every resume has both
        d.p([(addr[0], ""), ("\n", ""), (addr[1], "")], shape="br-run")
    else:
        d.p([("\n".join(addr), "")], shape="br-in-run")

    note = f"Two referees from {jobs[0]['company']} can be contacted after an interview."
    d.p([("References available on request.", "")], extra=FOOTNOTE_REF)

    header = para([(name, "b"), ("\t", ""), (email, ""), ("\t", ""), (phone, "")], style="Header")
    footer_text = f"Resume of {name} · Updated October 2026"
    footer = para([(footer_text, "")], style="Footer")
    d.present("header", f"{name} {email} {phone}")
    d.present("footer", footer_text)
    d.present("footnote", note)
    d.present("name", name)
    d.present("email", email)
    d.present("phone", phone)
    d.key["groups"]["jobs"] = job_lines
    d.key["fields"] = {"name": name, "email": email, "phone": phone, "jobs": job_fields, "education": edu,
                       "job_style": style}
    return docx_parts("".join(d.body), header, footer, note), d.key


def check_key(key):
    """score.py strips Markdown emphasis and escapes before matching, so no key string may hold them."""
    stack = [key]
    while stack:
        x = stack.pop()
        if isinstance(x, str):
            assert not re.search(r"[*_\\`]", x), x
        elif isinstance(x, (dict, list)):
            stack.extend(x.values() if isinstance(x, dict) else x)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--out", type=pathlib.Path, default=DEFAULT_OUT)
    out = ap.parse_args().out
    rng = random.Random(SEED)
    for i in range(COUNT):
        parts, key = resume(i, rng)
        check_key(key)
        write_package(out / key["source"], parts)
        (out / key["source"]).with_suffix(".key.json").write_text(
            json.dumps(key, ensure_ascii=False, indent=1, sort_keys=True) + "\n", encoding="utf-8")
    print(f"wrote {COUNT} resumes and keys to {out}")


if __name__ == "__main__":
    main()
