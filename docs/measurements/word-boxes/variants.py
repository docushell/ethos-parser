"""Build grounding variants from an engine artifact, for resolution tests only (word boxes are
proportional placeholders, NOT measurements)."""
import json, sys, re, copy, hashlib, os

src, outdir = sys.argv[1], sys.argv[2]
base = os.path.basename(src).replace(".ground.json", "")
g = json.load(open(src))
el = {e["id"]: e for e in g["elements"]}

def dump(obj, name):
    p = os.path.join(outdir, f"{base}.{name}.ground.json")
    b = json.dumps(obj, separators=(",", ":"), sort_keys=True, ensure_ascii=False).encode()
    open(p, "wb").write(b)
    return p

def run_offsets(art):
    cursor = {}
    bad = 0
    for s in art["spans"]:
        text = el[s["element"]]["text"]
        c = cursor.get(s["element"], 0)
        i = text.find(s["text"], c)
        if i < 0:
            bad += 1
            continue
        s["char_start"] = len(text[:i])  # python str index == scalar index
        s["char_end"] = i + len(s["text"])
        cursor[s["element"]] = s["char_end"]
    art["capabilities"]["char_offsets"] = True
    return bad

B = copy.deepcopy(g)
bad = run_offsets(B)
print("run offsets unplaced", bad)
dump(B, "B_run_offsets")

def word_spans(art, with_offsets):
    out = []
    for s in art["spans"]:
        out.append(s)
        st = s.get("char_start")
        text = s["text"]
        x0, y0, x1, y1 = s["bbox"]
        n = len(text)
        for k, m in enumerate(re.finditer(r"\S+", text)):
            if m.start() == 0 and m.end() == n:
                continue  # the run is the word
            a = x0 + (x1 - x0) * m.start() // n
            b = x0 + (x1 - x0) * m.end() // n
            if b <= a:
                b = a + 1
            w = {"id": f"{s['id']}.w{k}", "page": s["page"], "bbox": [a, y0, b, y1], "text": m.group(0), "element": s["element"]}
            if with_offsets:
                w["char_start"] = st + m.start()
                w["char_end"] = st + m.end()
            out.append(w)
    art["spans"] = out

C = copy.deepcopy(B)
word_spans(C, True)
dump(C, "C_word_offsets")
D = copy.deepcopy(B)
word_spans(D, True)
for s in D["spans"]:
    s.pop("char_start", None); s.pop("char_end", None)
D["capabilities"]["char_offsets"] = False
dump(D, "D_word_no_offsets")
N = copy.deepcopy(B)
N["spans"][1].pop("char_start"); N["spans"][1].pop("char_end")
dump(N, "NEG_mixed_offsets")
S = copy.deepcopy(B)
S["spans"][1]["char_start"] += 1; S["spans"][1]["char_end"] += 1
dump(S, "NEG_shifted")
E = copy.deepcopy(C)  # a word span whose box lies on another line of the page
for s in E["spans"]:
    if s["id"] == "s25.w6":
        s["bbox"] = [3600, 2981, 9621, 5866]
dump(E, "E_box_elsewhere")
print("spans A/B/C", len(g["spans"]), len(B["spans"]), len(C["spans"]))
for s in C["spans"]:
    if s["id"].startswith("s25"):
        print(s["id"], repr(s["text"]), s["bbox"], s.get("char_start"), s.get("char_end"))
