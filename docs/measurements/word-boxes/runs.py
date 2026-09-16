"""Classify each run in a grounded block against the words of that block.
Usage: runs.py REPRESENTATION.json GROUNDING.json"""
import json, sys, collections, re, statistics

def pct(xs, p):
    xs = sorted(xs)
    if not xs:
        return None
    k = max(0, min(len(xs) - 1, int(round(p / 100 * (len(xs) - 1)))))
    return xs[k]

rep_path, ground_path = sys.argv[1], sys.argv[2]
d = json.load(open(rep_path))
geom = {g["node"]: g["presence"]["state"] == "measured" for g in d["geometry"]}
nodes = d["representation"]["nodes"]
pos = {n["id"]: i for i, n in enumerate(nodes)}
g = json.load(open(ground_path))
del d
el_text = {e["id"]: e["text"] for e in g["elements"]}
el_spans = collections.defaultdict(list)
for s in g.get("spans", []):
    el_spans[s["element"]].append(s["id"])
groups = []
for eid, sids in el_spans.items():
    text = el_text[eid]
    first, last = pos[sids[0]], pos[sids[-1]]
    core = "".join(nodes[i]["text"] for i in range(first, last + 1))
    at = text.find(core)
    lo, need = first, at
    while need > 0 and lo > 0:
        lo -= 1
        need -= len(nodes[lo]["text"])
    hi, need = last, len(text) - at - len(core)
    while need > 0 and hi + 1 < len(nodes):
        hi += 1
        need -= len(nodes[hi]["text"])
    members = list(range(lo, hi + 1))
    if "".join(nodes[i]["text"] for i in members) == text:
        groups.append(members)

C = collections.Counter()
runs_per_word = []
chars_per_run, words_per_run = [], []
runs_per_page = collections.Counter()
for n in nodes:
    if n["kind"] == "text_run":
        chars_per_run.append(len(n["text"]))
        words_per_run.append(len(n["text"].split()))
        runs_per_page[n["native_locator"]["pdf"]["page"]] += 1
WORD = re.compile(r"\S+")
for members in groups:
    text = "".join(nodes[i]["text"] for i in members)
    words = [(m.start(), m.end()) for m in WORD.finditer(text)]
    off = 0
    wcount = collections.Counter()
    for i in members:
        t = nodes[i]["text"]
        s0, s1 = off, off + len(t)
        off = s1
        C["runs_in_grounded_blocks"] += 1
        nz = [p for p in range(s0, s1) if not text[p].isspace()]
        if not nz:
            C["whitespace_only"] += 1
            continue
        ov = [w for w in words if w[0] < s1 and w[1] > s0]
        for w in ov:
            wcount[w] += 1
        if len(ov) == 1:
            w = ov[0]
            if nz[0] == w[0] and nz[-1] == w[1] - 1:
                C["one_whole_word"] += 1
            else:
                C["fragment_of_a_word"] += 1
        else:
            if all(w[0] >= s0 and w[1] <= s1 for w in ov):
                C["several_whole_words"] += 1
            else:
                C["several_words_cut_at_an_end"] += 1
    runs_per_word.extend(wcount[w] for w in words)
name = rep_path.split("/")[-1]
print(name, dict(sorted(C.items())), "runs_per_word_median", statistics.median(runs_per_word) if runs_per_word else None,
      "p90", pct(runs_per_word, 90), "chars_per_run median/p90/max", statistics.median(chars_per_run), pct(chars_per_run, 90), max(chars_per_run),
      "words_per_run median/p90/max", statistics.median(words_per_run), pct(words_per_run, 90), max(words_per_run),
      "runs_per_page", sorted(runs_per_page.values())[len(runs_per_page)//2], max(runs_per_page.values()), "RPP", json.dumps(sorted(runs_per_page.values())))
