"""Runs with a positive advance at one origin x, and the state of their boxes. For a note drawn
along the page's edge under a rotated CTM, every glyph shares an origin x.
Usage: margin.py REPRESENTATION.json ORIGIN_X"""
import json, sys, collections
d = json.load(open(sys.argv[1])); x = int(sys.argv[2])
state = {g["node"]: g["presence"]["state"] for g in d["geometry"]}
c = collections.Counter(); pages = set()
for n in d["representation"]["nodes"]:
    if n["kind"] != "text_run": continue
    loc = n["native_locator"]["pdf"]
    if loc["origin_x"] == x and (loc.get("advance") or 0) > 0:
        c[state[n["id"]]] += 1; pages.add(loc["page"])
print(dict(c), "pages", len(pages))
