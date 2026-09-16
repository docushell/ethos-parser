import zlib

def pdf(objs, path):
    # objs: list of bytes bodies for objects 1..n; obj 1 = catalog
    out = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
    offs = []
    for i, body in enumerate(objs, 1):
        offs.append(len(out))
        out += b"%d 0 obj\n" % i + body + b"\nendobj\n"
    xref = len(out)
    out += b"xref\n0 %d\n" % (len(objs) + 1)
    out += b"0000000000 65535 f \n"
    for o in offs:
        out += b"%010d 00000 n \n" % o
    out += b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (len(objs) + 1, xref)
    open(path, "wb").write(out)

def stream(data, extra=b""):
    return b"<< /Length %d %s>>\nstream\n" % (len(data), extra) + data + b"\nendstream"

W500 = b"[" + b" ".join([b"500"] * 95) + b"]"

def simple_font_objs(start, tounicode=None, lastchar=126, encoding=b"/WinAnsiEncoding"):
    # returns (font_obj_body, descriptor_body, [tounicode body])
    desc = b"<< /Type /FontDescriptor /FontName /ProbeSans /Flags 32 /FontBBox [-100 -207 1000 718] /ItalicAngle 0 /Ascent 718 /Descent -207 /CapHeight 700 /StemV 80 >>"
    widths = b"[" + b" ".join([b"500"] * (lastchar - 32 + 1)) + b"]"
    tu = b""
    if tounicode is not None:
        tu = b" /ToUnicode %d 0 R" % (start + 2)
    font = b"<< /Type /Font /Subtype /TrueType /BaseFont /ProbeSans /FirstChar 32 /LastChar %d /Widths %s /Encoding %s /FontDescriptor %d 0 R%s >>" % (lastchar, widths, encoding, start + 1, tu)
    objs = [font, desc]
    if tounicode is not None:
        objs.append(stream(tounicode))
    return objs

def page_doc(path, content, rotate=0, font_objs=None, media=b"[0 0 400 400]"):
    # 1 catalog, 2 pages, 3 page, 4 content, 5.. font
    if font_objs is None:
        font_objs = simple_font_objs(5)
    rot = b" /Rotate %d" % rotate if rotate else b""
    objs = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox %s%s /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>" % (media, rot),
        stream(content),
    ] + font_objs
    pdf(objs, path)

P = {}
P["p01-plain"] = b"BT /F1 10 Tf 50 50 Td (abcde) Tj ET"
P["p02-tc2"] = b"BT /F1 10 Tf 2 Tc 50 50 Td (abcde) Tj ET"
P["p03-tw4"] = b"BT /F1 10 Tf 4 Tw 50 50 Td (ab de) Tj ET"
P["p04-tz50-tc2"] = b"BT /F1 10 Tf 50 Tz 2 Tc 50 50 Td (abcde) Tj ET"
P["p05-ts5"] = b"BT /F1 10 Tf 5 Ts 50 50 Td (abcde) Tj ET"
P["p06-tf1-tm10"] = b"BT /F1 1 Tf 10 0 0 10 50 50 Tm (abcde) Tj ET"
P["p07-tm-rot90"] = b"BT /F1 10 Tf 0 1 -1 0 50 50 Tm (abcde) Tj ET"
P["p08-tm-rot180"] = b"BT /F1 10 Tf -1 0 0 -1 250 50 Tm (abcde) Tj ET"
P["p09-tm-rot45"] = b"BT /F1 10 Tf 0.70710678 0.70710678 -0.70710678 0.70710678 50 50 Tm (abcde) Tj ET"
P["p10-ctm-rot90"] = b"q 0 1 -1 0 200 0 cm BT /F1 10 Tf 50 50 Td (abcde) Tj ET Q"
P["p11-ctm05-tf20"] = b"q 0.5 0 0 0.5 0 0 cm BT /F1 20 Tf 100 100 Td (abcde) Tj ET Q"
P["p12-tj"] = b"BT /F1 10 Tf 50 50 Td [(W) 80 (ord) -300 (next) -100 (word)] TJ ET"
P["p15-mirror"] = b"BT /F1 10 Tf -1 0 0 1 250 50 Tm (abcde) Tj ET"
P["p16-td-gap"] = b"BT /F1 10 Tf 50 50 Td (ab) Tj 20 0 Td (cd) Tj ET"
P["p25-tj-number-first"] = b"BT /F1 10 Tf 50 50 Td (ab) Tj ET BT /F1 10 Tf 50 100 Td [-400 (cd)] TJ ET"
for name, c in P.items():
    page_doc(name + ".pdf", c)
page_doc("p17-rotate90.pdf", P["p01-plain"], rotate=90)
page_doc("p24-rotate180.pdf", P["p01-plain"], rotate=180)
page_doc("p24-rotate270.pdf", P["p01-plain"], rotate=270)
# p13: code 233 beyond LastChar 126
page_doc("p13-beyond-lastchar.pdf", b"BT /F1 10 Tf 50 50 Td (caf\xe9 ok) Tj ET")
# p18: ToUnicode maps 'd' -> "fi"
cmap = b"""/CIDInit /ProcSet findresource begin 12 dict begin begincmap
/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def
/CMapName /Adobe-Identity-UCS def /CMapType 2 def
1 begincodespacerange <00> <FF> endcodespacerange
5 beginbfchar
<61> <0061>
<62> <0062>
<20> <0020>
<63> <0063>
<64> <00660069>
endbfchar
endcmap CMapName currentdict /CMap defineresource pop end end"""
page_doc("p18-ligature.pdf", b"BT /F1 10 Tf 50 50 Td (ab cd) Tj ET", font_objs=simple_font_objs(5, tounicode=cmap))
# p20: Type3 FontMatrix 0.01, descriptor 70/-20
t3font = b"<< /Type /Font /Subtype /Type3 /FontBBox [0 -20 50 70] /FontMatrix [0.01 0 0 0.01 0 0] /CharProcs << /a 7 0 R >> /Encoding << /Type /Encoding /Differences [97 /a /a /a /a /a] >> /FirstChar 97 /LastChar 101 /Widths [50 50 50 50 50] /FontDescriptor 6 0 R /Resources << >> >>"
t3desc = b"<< /Type /FontDescriptor /FontName /T3 /Flags 4 /FontBBox [0 -20 50 70] /ItalicAngle 0 /Ascent 70 /Descent -20 /CapHeight 70 /StemV 10 >>"
t3proc = stream(b"50 0 0 -20 50 70 d1 0 0 50 70 re f")
page_doc("p20-type3.pdf", b"BT /F1 10 Tf 50 50 Td (abcde) Tj ET", font_objs=[t3font, t3desc, t3proc])
# p22: Identity-V Type0
cid_cmap = b"""/CIDInit /ProcSet findresource begin 12 dict begin begincmap
/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def
/CMapName /Adobe-Identity-UCS def /CMapType 2 def
1 begincodespacerange <0000> <FFFF> endcodespacerange
3 beginbfchar
<0001> <0041>
<0002> <0042>
<0003> <0043>
endbfchar
endcmap CMapName currentdict /CMap defineresource pop end end"""
t0 = b"<< /Type /Font /Subtype /Type0 /BaseFont /ProbeCID /Encoding /Identity-V /DescendantFonts [6 0 R] /ToUnicode 8 0 R >>"
cidf = b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /ProbeCID /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 7 0 R /DW 500 /W [1 [500 500 500]] >>"
cdesc = b"<< /Type /FontDescriptor /FontName /ProbeCID /Flags 4 /FontBBox [0 -207 1000 718] /ItalicAngle 0 /Ascent 718 /Descent -207 /CapHeight 700 /StemV 80 >>"
page_doc("p22-identity-v.pdf", b"BT /F1 10 Tf 50 350 Td <000100020003> Tj ET", font_objs=[t0, cidf, cdesc, stream(cid_cmap)])
# p26: Identity-H with word spacing and a TWO-byte code <0020>; PDF 32000-1 9.3.3 applies Tw to a
# single-byte code 32 only
cid_cmap_h = b"""/CIDInit /ProcSet findresource begin 12 dict begin begincmap
/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def
/CMapName /Adobe-Identity-UCS def /CMapType 2 def
1 begincodespacerange <0000> <FFFF> endcodespacerange
2 beginbfchar
<0001> <0041>
<0020> <0042>
endbfchar
endcmap CMapName currentdict /CMap defineresource pop end end"""
t0h = t0.replace(b"/Identity-V", b"/Identity-H")
cidf_dw = b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /ProbeCID /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 7 0 R /DW 500 >>"
page_doc("p26-tw-twobyte32.pdf", b"BT /F1 10 Tf 10 Tw 50 350 Td <000100200001> Tj ET", font_objs=[t0h, cidf_dw, cdesc, stream(cid_cmap_h)])
# p27: a run holding one width-less code, then an ordinary run
page_doc("p27-widthless-then-next.pdf", b"BT /F1 10 Tf 50 350 Td (caf\xe9 ok) Tj 30 0 Td (next) Tj ET")
print("ok")
