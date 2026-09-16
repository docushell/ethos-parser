"""Same-baseline joins between consecutive measured runs that carry no whitespace, by gap in box heights,
and how many of the large gaps lie between two runs of dots (leaders). Usage: joins.py REPRESENTATION.json ..."""
import json, sys, collections
T = collections.Counter()
for path in sys.argv[1:]:
    d = json.load(open(path))
    geom = {g["node"]: g["presence"] for g in d["geometry"]}
    nodes = d["representation"]["nodes"]
    C = collections.Counter()
    prev = None
    for n in nodes:
        if n["kind"] != "text_run":
            prev = None; continue
        loc = n["native_locator"]["pdf"]
        if prev is not None:
            pl = prev["native_locator"]["pdf"]
            if (pl["page"], pl["origin_y"]) == (loc["page"], loc["origin_y"]):
                C["joins"] += 1
                if prev["text"] and n["text"] and not prev["text"][-1].isspace() and not n["text"][0].isspace():
                    gp, gn = geom[prev["id"]], geom[n["id"]]
                    if gp["state"] == "measured" and gn["state"] == "measured":
                        C["ws_free_joins"] += 1
                        a, b = gp["value"], gn["value"]
                        for hname, h in (("hprev", a[3]-a[1]), ("hmax", max(a[3]-a[1], b[3]-b[1]))):
                            gap = (b[0] - a[2]) / h
                            if abs(gap) <= 0.02: C[hname+"_|gap|<=0.02h"] += 1
                            if gap >= 0.25:
                                C[hname+"_gap>=0.25h"] += 1
                                if hname == "hmax" and set(prev["text"].strip()) == {"."} and set(n["text"].strip()) == {"."}:
                                    C["hmax_gap>=0.25h_between_two_dot_runs"] += 1
                            elif gap >= 0.1: C[hname+"_0.1h<=gap<0.25h"] += 1
        prev = n
    print(path, dict(sorted(C.items())))
    T.update(C)
print("TOTAL", dict(sorted(T.items())))
