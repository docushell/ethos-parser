"""What qpdf says about every corpus PDF: encryption, page count, and declared colour spaces.

Usage:
  qpdf_census.py run [--out FILE] [CORPUS ...]
  qpdf_census.py summary [FILE]

Out-of-band inspection with qpdf (Apache-2.0), independent of the engine, so the engine's refusal
census can be checked against a second reader. One TSV row per PDF:

  encrypted          `qpdf --is-encrypted` exit 0
  needs_secret       `qpdf --requires-password` exit 0: a password other than the empty one is
                     required. Exit 3 means the file is encrypted and the empty user password
                     opens it — the only encrypted case a password knob could open without a secret
  enc_method, enc_r  from `qpdf --show-encryption` (file encryption method, and R)
  pages              `qpdf --show-npages`
  page_cs            colour-space families declared in page `/Resources /ColorSpace` dictionaries,
                     inherited `/Resources` included, as `family=count;...` over all pages, where
                     the count is the number of (page, entry) pairs
  resource_cs        the same over every `/ColorSpace` resource dictionary anywhere in the file:
                     pages, form XObjects, tiling patterns, Type 3 fonts
  image_cs           the `/ColorSpace` of every image XObject, by family
  patterns, shadings entries in `/Pattern` and `/Shading` resource dictionaries anywhere in the file

A family is the colour space's own name for a device space (DeviceGray, DeviceRGB, DeviceCMYK,
Pattern) or the first element of its array (ICCBased, Indexed, Separation, DeviceN, CalRGB, CalGray,
Lab, Pattern). Indexed, Separation, DeviceN and Pattern are written with their base or alternate
family after a colon, e.g. `Indexed:ICCBased`, so the resolution cost of each entry is visible.
Nothing here counts operator use in content streams: a document can paint with `rg` and declare no
colour space at all, and such a document shows an empty `page_cs`.

An encrypted file is inspected with `--password=`; one the empty password does not open is
recorded `unreadable` in every column qpdf could not fill.

Environment: QPDF (default: qpdf on PATH), plus corpora.py's.
"""
import collections
import json
import os
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import corpora  # noqa: E402

QPDF = os.environ.get("QPDF") or "qpdf"

COLUMNS = ["corpus", "doc", "bytes", "encrypted", "needs_secret", "enc_method", "enc_r", "pages",
           "page_cs", "resource_cs", "image_cs", "patterns", "shadings"]

DEVICE = {"/DeviceGray", "/DeviceRGB", "/DeviceCMYK", "/Pattern", "/G", "/RGB", "/CMYK", "/I", "/Indexed"}


def qpdf(args, pdf):
    return subprocess.run([QPDF] + args + [pdf], capture_output=True, text=True)


class Objects:
    """The `qpdf --json=2` object table, with reference resolution."""

    def __init__(self, doc):
        self.objs = {}
        for chunk in doc["qpdf"]:
            for k, v in chunk.items():
                if k.startswith("obj:") or k == "trailer":
                    self.objs[k[4:] if k.startswith("obj:") else k] = v

    def resolve(self, v, depth=0):
        while isinstance(v, str) and v.endswith(" R") and depth < 32:
            o = self.objs.get(v)
            if o is None:
                return None
            v = o["value"] if "value" in o else o["stream"]["dict"]
            depth += 1
        return v

    def dict_of(self, v):
        v = self.resolve(v)
        return v if isinstance(v, dict) else None


def family(objs, cs, depth=0):
    """The family name of one colour-space object, with its base for the four that carry one."""
    cs = objs.resolve(cs)
    if isinstance(cs, str):
        return cs.lstrip("/") if cs.startswith("/") else "?" + cs
    if isinstance(cs, list) and cs:
        head = objs.resolve(cs[0])
        name = head.lstrip("/") if isinstance(head, str) else "?"
        if depth < 4:
            if name == "Indexed" and len(cs) > 1:
                return "Indexed:" + family(objs, cs[1], depth + 1)
            if name in ("Separation", "DeviceN") and len(cs) > 2:
                return name + ":" + family(objs, cs[2], depth + 1)
            if name == "Pattern" and len(cs) > 1:
                return "Pattern:" + family(objs, cs[1], depth + 1)
        return name
    if isinstance(cs, dict):
        return "?dict"
    return "?"


def fmt(counter):
    return ";".join("%s=%d" % (k, v) for k, v in sorted(counter.items()))


def page_resources(objs, page):
    """A page's `/Resources`, walking `/Parent` when the page dictionary carries none."""
    d = page
    seen = 0
    while isinstance(d, dict) and seen < 64:
        if "/Resources" in d:
            return objs.dict_of(d["/Resources"]) or {}
        d = objs.dict_of(d.get("/Parent"))
        seen += 1
    return {}


def colour_census(pdf, password_arg):
    p = subprocess.run([QPDF] + password_arg + ["--json=2", "--json-key=qpdf", "--json-key=pages", pdf],
                       capture_output=True, text=True)
    if p.returncode not in (0, 3) or not p.stdout:  # 3: warnings, output still produced
        return None
    doc = json.loads(p.stdout)
    objs = Objects(doc)

    page_cs = collections.Counter()
    for page in doc.get("pages", []):
        pd = objs.dict_of(page["object"])
        if pd is None:
            continue
        csd = objs.dict_of(page_resources(objs, pd).get("/ColorSpace"))
        for name, cs in (csd or {}).items():
            page_cs[family(objs, cs)] += 1

    resource_cs, image_cs = collections.Counter(), collections.Counter()
    counts = {"patterns": 0, "shadings": 0}

    def visit(d):
        # One dictionary, wherever it sits: an object's own dictionary, or one written inline
        # inside another (a page's `/Resources` is usually inline). References are not followed
        # here because every referenced dictionary is an object of its own and is visited as one.
        if objs.resolve(d.get("/Subtype")) == "/Image":
            if "/ColorSpace" in d:
                image_cs[family(objs, d["/ColorSpace"])] += 1
            return
        cs = d.get("/ColorSpace")
        csd = objs.dict_of(cs) if cs is not None and not (isinstance(cs, str) and cs.startswith("/")) and not isinstance(cs, list) else None
        if csd:
            for name, entry in csd.items():
                resource_cs[family(objs, entry)] += 1
        pats = d.get("/Pattern")
        patd = objs.dict_of(pats) if pats is not None and not isinstance(pats, list) else None
        if patd and "/PatternType" not in patd:
            counts["patterns"] += len(patd)
        shs = d.get("/Shading")
        shd = objs.dict_of(shs) if shs is not None else None
        if shd and "/ShadingType" not in shd:
            counts["shadings"] += len(shd)

    def walk(v):
        if isinstance(v, dict):
            visit(v)
            for x in v.values():
                walk(x)
        elif isinstance(v, list):
            for x in v:
                walk(x)

    for key, o in objs.objs.items():
        walk(o["value"] if "value" in o else o["stream"]["dict"])
    patterns, shadings = counts["patterns"], counts["shadings"]
    return fmt(page_cs), fmt(resource_cs), fmt(image_cs), patterns, shadings


def run_one(corpus, path):
    row = dict((c, "") for c in COLUMNS)
    row["corpus"], row["doc"], row["bytes"] = corpus, corpora.label(corpus, path), os.path.getsize(path)
    row["encrypted"] = int(qpdf(["--is-encrypted"], path).returncode == 0)
    rp = qpdf(["--requires-password"], path).returncode
    row["needs_secret"] = {0: 1, 2: 0, 3: 0}.get(rp, "?%d" % rp)
    pw = ["--password="] if row["encrypted"] else []
    if row["encrypted"]:
        se = qpdf(pw + ["--show-encryption"], path).stdout
        for line in se.splitlines():
            if line.startswith("file encryption method:"):
                row["enc_method"] = line.split(":", 1)[1].strip()
            if line.startswith("R = "):
                row["enc_r"] = line[4:].strip()
    np_ = qpdf(pw + ["--show-npages"], path)
    row["pages"] = np_.stdout.strip() if np_.returncode == 0 else "unreadable"
    c = colour_census(path, pw)
    if c is None:
        row["page_cs"] = row["resource_cs"] = row["image_cs"] = row["patterns"] = row["shadings"] = "unreadable"
    else:
        row["page_cs"], row["resource_cs"], row["image_cs"], row["patterns"], row["shadings"] = c
    return row


def cmd_run(argv):
    out, only, i = None, [], 0
    while i < len(argv):
        if argv[i] == "--out":
            out, i = argv[i + 1], i + 2
        else:
            only.append(argv[i])
            i += 1
    if out is None:
        out = os.path.join(os.path.dirname(os.path.abspath(__file__)), "results", "qpdf-census.tsv")
    os.makedirs(os.path.dirname(out), exist_ok=True)
    docs = corpora.pdfs(only or None)
    with open(out, "w") as f:
        f.write("\t".join(COLUMNS) + "\n")
        for n, (corpus, path) in enumerate(docs, 1):
            row = run_one(corpus, path)
            f.write("\t".join(str(row[c]) for c in COLUMNS) + "\n")
            f.flush()
            print("%d/%d %s %s pages=%s enc=%s" % (n, len(docs), corpus, row["doc"], row["pages"], row["encrypted"]), file=sys.stderr)
    print("wrote %s (%d rows)" % (out, len(docs)), file=sys.stderr)


def read_tsv(path):
    with open(path) as f:
        header = f.readline().rstrip("\n").split("\t")
        return [dict(zip(header, line.rstrip("\n").split("\t"))) for line in f if line.strip()]


def parse_counts(s):
    c = collections.Counter()
    if s and s != "unreadable":
        for part in s.split(";"):
            k, v = part.rsplit("=", 1)
            c[k] += int(v)
    return c


def cmd_summary(argv):
    path = argv[0] if argv else os.path.join(os.path.dirname(os.path.abspath(__file__)), "results", "qpdf-census.tsv")
    rows = read_tsv(path)
    by = collections.OrderedDict()
    for r in rows:
        by.setdefault(r["corpus"], []).append(r)

    print("encryption and page counts")
    print("corpus\tdocs\tencrypted\tneeds_secret\tempty_user_password\tpages_total\tpages_max\tdocs_over_50_pages\tdocs_over_8_pages\tdocs_1_page")
    tot = collections.Counter()
    for corpus, rs in by.items():
        c = collections.Counter()
        c["docs"] = len(rs)
        c["encrypted"] = sum(1 for r in rs if r["encrypted"] == "1")
        c["needs_secret"] = sum(1 for r in rs if r["needs_secret"] == "1")
        c["empty_user_password"] = sum(1 for r in rs if r["encrypted"] == "1" and r["needs_secret"] == "0")
        pages = [int(r["pages"]) for r in rs if r["pages"].isdigit()]
        c["pages_total"] = sum(pages)
        c["pages_max"] = max(pages) if pages else 0
        c["docs_over_50_pages"] = sum(1 for p in pages if p > 50)
        c["docs_over_8_pages"] = sum(1 for p in pages if p > 8)
        c["docs_1_page"] = sum(1 for p in pages if p == 1)
        tot.update(c)
        tot["pages_max"] = max(tot["pages_max"], c["pages_max"])
        print("\t".join([corpus] + [str(c[k]) for k in ("docs", "encrypted", "needs_secret", "empty_user_password", "pages_total", "pages_max", "docs_over_50_pages", "docs_over_8_pages", "docs_1_page")]))
    print("\t".join(["ALL"] + [str(tot[k]) for k in ("docs", "encrypted", "needs_secret", "empty_user_password", "pages_total", "pages_max", "docs_over_50_pages", "docs_over_8_pages", "docs_1_page")]))

    print("\nencrypted documents:")
    for r in rows:
        if r["encrypted"] == "1":
            print("  %s\t%s\tneeds_secret=%s\tmethod=%s\tR=%s\tpages=%s" % (r["corpus"], r["doc"], r["needs_secret"], r["enc_method"], r["enc_r"], r["pages"]))

    print("\ndocuments over 50 pages:")
    for r in rows:
        if r["pages"].isdigit() and int(r["pages"]) > 50:
            print("  %s\t%s\t%s" % (r["corpus"], r["doc"], r["pages"]))

    for col, title in (("page_cs", "colour-space families declared in page resources (entries; documents declaring)"),
                       ("resource_cs", "colour-space families in every resource dictionary (entries; documents)"),
                       ("image_cs", "image XObject colour spaces (images; documents)")):
        print("\n%s" % title)
        entries, docs_with = collections.Counter(), collections.Counter()
        for r in rows:
            c = parse_counts(r[col])
            entries.update(c)
            for k in c:
                docs_with[k] += 1
        print("family\tentries\tdocuments")
        for k, v in sorted(entries.items(), key=lambda kv: -kv[1]):
            print("%s\t%d\t%d" % (k, v, docs_with[k]))
        readable = [r for r in rows if r[col] != "unreadable"]
        none = sum(1 for r in readable if r[col] == "")
        print("documents readable: %d; declaring none: %d" % (len(readable), none))

    print("\nper corpus: documents whose page resources declare any non-device colour space (ICCBased, Indexed, Separation, DeviceN, Cal*, Lab), any Pattern resource, any Shading resource:")
    print("corpus\tdocs\tnon_device_page_cs\tpattern_or_shading_cs_in_page\tpatterns\tshadings")
    for corpus, rs in by.items():
        nd = sum(1 for r in rs if any(not k.startswith("Device") and not k.startswith("Pattern") for k in parse_counts(r["page_cs"])))
        pat = sum(1 for r in rs if any(k.startswith("Pattern") for k in parse_counts(r["page_cs"])))
        pats = sum(1 for r in rs if r["patterns"] not in ("", "0", "unreadable"))
        shs = sum(1 for r in rs if r["shadings"] not in ("", "0", "unreadable"))
        print("%s\t%d\t%d\t%d\t%d\t%d" % (corpus, len(rs), nd, pat, pats, shs))


if __name__ == "__main__":
    if len(sys.argv) < 2 or sys.argv[1] not in ("run", "summary"):
        print(__doc__)
        sys.exit(2)
    {"run": cmd_run, "summary": cmd_summary}[sys.argv[1]](sys.argv[2:])
