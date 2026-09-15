"""Element box area over the union of the run boxes covering a word, per word and per five-word window.
Trimmed-union figures are estimates. Usage: areas.py REPRESENTATION.json GROUNDING.json"""
import json, sys, collections, re, statistics

rep_path, ground_path = sys.argv[1], sys.argv[2]
d = json.load(open(rep_path))
box = {g["node"]: g["presence"].get("value") if g["presence"]["state"] == "measured" else None for g in d["geometry"]}
nodes = d["representation"]["nodes"]
pos = {n["id"]: i for i, n in enumerate(nodes)}
g = json.load(open(ground_path))
del d
el = {e["id"]: e for e in g["elements"]}
el_spans = collections.defaultdict(list)
for s in g.get("spans", []):
    el_spans[s["element"]].append(s["id"])

def area(b):
    return (b[2] - b[0]) * (b[3] - b[1])

def union(bs):
    return [min(b[0] for b in bs), min(b[1] for b in bs), max(b[2] for b in bs), max(b[3] for b in bs)]

r_word, r_trim_word, r_q5, r_q5_trim = [], [], [], []
for eid, sids in el_spans.items():
    e = el[eid]
    text = e["text"]
    first, last = pos[sids[0]], pos[sids[-1]]
    core = "".join(nodes[i]["text"] for i in range(first, last + 1))
    at = text.find(core)
    lo, need = first, at
    while need > 0 and lo > 0:
        lo -= 1; need -= len(nodes[lo]["text"])
    hi, need = last, len(text) - at - len(core)
    while need > 0 and hi + 1 < len(nodes):
        hi += 1; need -= len(nodes[hi]["text"])
    members = list(range(lo, hi + 1))
    if "".join(nodes[i]["text"] for i in members) != text:
        continue
    spans = []; off = 0
    for i in members:
        t = nodes[i]["text"]; spans.append((off, off + len(t), i)); off += len(t)
    words = [(m.start(), m.end()) for m in re.finditer(r"\S+", text)]
    ea = area(e["bbox"])

    def measure(ws, we):
        cover = [s for s in spans if s[0] < we and s[1] > ws]
        bs = [box[nodes[i]["id"]] for _, _, i in cover if box[nodes[i]["id"]]]
        if not bs:
            return None
        u = union(bs)
        # proportional trim by scalar count (ESTIMATE)
        tb = []
        for s0, s1, i in cover:
            b = box[nodes[i]["id"]]
            if not b:
                continue
            n = s1 - s0
            a = max(ws, s0) - s0; z = min(we, s1) - s0
            tb.append([b[0] + (b[2] - b[0]) * a / n, b[1], b[0] + (b[2] - b[0]) * z / n, b[3]])
        t = union(tb)
        return u, t

    for ws, we in words:
        m = measure(ws, we)
        if not m:
            continue
        u, t = m
        if area(u) > 0:
            r_word.append(ea / area(u))
        if area(t) > 0:
            r_trim_word.append(area(u) / area(t))
    for k in range(0, len(words) - 4, 5):
        ws, we = words[k][0], words[k + 4][1]
        m = measure(ws, we)
        if not m:
            continue
        u, t = m
        if area(u) > 0:
            r_q5.append(ea / area(u))
        if area(t) > 0:
            r_q5_trim.append(area(u) / area(t))
med = lambda xs: round(statistics.median(xs), 3) if xs else None
print(rep_path.split("/")[-1], "elem/spanunion per word", med(r_word), "q5 elem/spanunion", med(r_q5),
      "EST spanunion/trimmed word", med(r_trim_word), "EST q5 spanunion/trimmed median", med(r_q5_trim),
      "mean", round(statistics.mean(r_q5_trim), 3) if r_q5_trim else None, "q5 windows", len(r_q5))
