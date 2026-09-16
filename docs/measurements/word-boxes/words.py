"""Classify whitespace-delimited words against the runs that draw them.

Grouping G = 'element': grounding elements (geometric blocks); membership rebuilt from the
representation by walking nodes in order and matching element text greedily.
Grouping G = 'line': consecutive text runs in node order with the same page and origin_y.
"""
import json, sys, collections, re

rep_path, ground_path = sys.argv[1], sys.argv[2]
d = json.load(open(rep_path))
geom = {g["node"]: g["presence"]["state"] == "measured" for g in d["geometry"]}
nodes = d["representation"]["nodes"]
pos = {n["id"]: i for i, n in enumerate(nodes)}
g = json.load(open(ground_path))
del d

# ---- element membership: every span's node, plus the unmeasured runs between/around them that
# ---- make the element text match when concatenated.
el_text = {e["id"]: e["text"] for e in g["elements"]}
el_spans = collections.defaultdict(list)
for s in g.get("spans", []):
    el_spans[s["element"]].append(s["id"])
groups_el = []
fail = 0
for eid, sids in el_spans.items():
    text = el_text[eid]
    first, last = pos[sids[0]], pos[sids[-1]]
    core = "".join(nodes[i]["text"] for i in range(first, last + 1))
    at = text.find(core)
    if at < 0:
        fail += 1
        continue
    lo, need = first, at
    while need > 0 and lo > 0:
        lo -= 1
        need -= len(nodes[lo]["text"])
    hi, need = last, len(text) - at - len(core)
    while need > 0 and hi + 1 < len(nodes):
        hi += 1
        need -= len(nodes[hi]["text"])
    members = list(range(lo, hi + 1))
    if "".join(nodes[i]["text"] for i in members) != text:
        fail += 1
        continue
    groups_el.append(members)

# ---- line grouping
groups_line = []
cur, key = [], None
for i, n in enumerate(nodes):
    if n["kind"] != "text_run":
        if cur:
            groups_line.append(cur)
        cur, key = [], None
        continue
    k = (n["native_locator"]["pdf"]["page"], n["native_locator"]["pdf"]["origin_y"])
    if k != key and cur:
        groups_line.append(cur)
        cur = []
    cur.append(i)
    key = k
if cur:
    groups_line.append(cur)

WORD = re.compile(r"\S+")

def classify(groups):
    C = collections.Counter()
    for members in groups:
        spans = []  # (start, end, idx)
        off = 0
        for i in members:
            t = nodes[i]["text"]
            spans.append((off, off + len(t), i))
            off += len(t)
        text = "".join(nodes[i]["text"] for i in members)
        for m in WORD.finditer(text):
            ws, we = m.start(), m.end()
            cover = [s for s in spans if s[0] < we and s[1] > ws]
            C["words"] += 1
            if not all(geom[nodes[s[2]]["id"]] for s in cover):
                C["words_touching_unmeasured"] += 1
                continue
            C["words_all_measured"] += 1
            # non-ws outside the word inside covering runs?
            nonws_out = False
            any_out = False
            for s0, s1, i in cover:
                for p in range(s0, s1):
                    if p < ws or p >= we:
                        any_out = True
                        if not text[p].isspace():
                            nonws_out = True
            if len(cover) == 1:
                if not any_out:
                    C["one_run_exact"] += 1
                elif not nonws_out:
                    C["one_run_modulo_ws"] += 1
                else:
                    C["inside_one_multiword_run"] += 1
            else:
                C["several_runs"] += 1
                if nonws_out:
                    C["several_runs_cut_nonws"] += 1
                elif any_out:
                    C["several_runs_cut_ws_only"] += 1
                else:
                    C["several_runs_whole_exact"] += 1
            if nonws_out:
                C["need_subrun_nonws"] += 1
            if any_out:
                C["need_subrun_strict"] += 1
    return C

name = rep_path.split("/")[-1]
print(name, "element_groups", len(groups_el), "element_membership_failures", fail)
print(name, "ELEMENT", dict(sorted(classify(groups_el).items())))
print(name, "LINE", dict(sorted(classify(groups_line).items())))
