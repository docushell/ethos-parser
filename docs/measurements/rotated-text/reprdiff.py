#!/usr/bin/env python3
"""Compare two `ethos-parser extract` artifacts node by node and categorise every difference.

usage: reprdiff.py BASE.json NEW.json [--samples N]

Prints one JSON line: counts per category and a few sample node ids per category.
Identity fields (parser_version, profile_sha256, representation_c14n_sha256) are reported
separately, because they are expected to move with any output change.
"""
import json
import sys
from collections import Counter, defaultdict


def load(p):
    with open(p, "rb") as f:
        return json.load(f)


def geom_map(doc):
    return {g["node"]: g["presence"] for g in doc.get("geometry", [])}


def presence_key(p):
    if p is None:
        return "none"
    if p["state"] == "measured":
        return "measured"
    return "absent:" + str(p.get("value"))


def main():
    base, new = sys.argv[1], sys.argv[2]
    samples = 3
    if "--samples" in sys.argv:
        samples = int(sys.argv[sys.argv.index("--samples") + 1])
    a, b = load(base), load(new)
    ra, rb = a["representation"], b["representation"]
    out = {"file": new.rsplit("/", 1)[-1]}
    cats = Counter()
    ex = defaultdict(list)

    def hit(cat, nid):
        cats[cat] += 1
        if len(ex[cat]) < samples:
            ex[cat].append(nid)

    na, nb = ra["nodes"], rb["nodes"]
    out["nodes"] = [len(na), len(nb)]
    ga, gb = geom_map(a), geom_map(b)
    ia = {n["id"]: n for n in na}
    ib = {n["id"]: n for n in nb}
    for nid in ia.keys() - ib.keys():
        hit("node_removed", nid)
    for nid in ib.keys() - ia.keys():
        hit("node_added", nid)
    for nid, x in ia.items():
        y = ib.get(nid)
        if y is None:
            continue
        if x.get("kind") != y.get("kind"):
            hit("kind", nid)
        if x.get("text") != y.get("text"):
            hit("text", nid)
        if x.get("parent") != y.get("parent"):
            hit("parent", nid)
        if x.get("ordinal") != y.get("ordinal"):
            hit("ordinal", nid)
        xa, ya = x.get("attributes", {}), y.get("attributes", {})
        if xa != ya:
            xt, yt = xa.get("text_run"), ya.get("text_run")
            if xt is not None and yt is not None:
                for k in sorted(set(xt) | set(yt)):
                    if xt.get(k) != yt.get(k):
                        hit("attr:text_run." + k, nid)
            else:
                hit("attributes:other", nid)
        xl, yl = x.get("native_locator", {}), y.get("native_locator", {})
        if xl != yl:
            xp, yp = xl.get("pdf"), yl.get("pdf")
            if xp is not None and yp is not None:
                for k in sorted(set(xp) | set(yp)):
                    if xp.get(k) != yp.get(k):
                        hit("locator:" + k, nid)
            else:
                hit("locator:other", nid)
        pa, pb = ga.get(nid), gb.get(nid)
        if pa != pb:
            ka, kb = presence_key(pa), presence_key(pb)
            if ka == kb == "measured":
                va, vb = pa["value"], pb["value"]
                wa, ha = va[2] - va[0], va[3] - va[1]
                wb, hb = vb[2] - vb[0], vb[3] - vb[1]
                shape = "moved"
                if (wa, ha) != (wb, hb):
                    shape = "reshaped"
                if (wa > ha) != (wb > hb):
                    shape = "reoriented"
                hit("geom:measured->measured:" + shape, nid)
            else:
                hit("geom:%s->%s" % (ka, kb), nid)
    for key in ("pages", "tables"):
        if ra.get(key) != rb.get(key):
            hit("repr:" + key, key)
    asa, asb = ra.get("assurance", {}), rb.get("assurance", {})
    for k in sorted(set(asa) | set(asb)):
        if asa.get(k) != asb.get(k):
            if k == "limitations":
                la = {json.dumps(l, sort_keys=True) for l in asa.get(k, [])}
                lb = {json.dumps(l, sort_keys=True) for l in asb.get(k, [])}
                for l in sorted(la - lb):
                    hit("limitation_removed", json.loads(l)["code"])
                for l in sorted(lb - la):
                    hit("limitation_added", json.loads(l)["code"])
            else:
                hit("assurance:" + k, k)
    for k in sorted(set(ra) - {"nodes", "pages", "tables", "assurance", "identity"}):
        if ra.get(k) != rb.get(k):
            hit("repr:" + k, k)
    idd = {k: [ra["identity"].get(k), rb["identity"].get(k)] for k in ra["identity"] if ra["identity"].get(k) != rb["identity"].get(k)}
    out["identity_moved"] = sorted(idd)
    out["categories"] = dict(sorted(cats.items()))
    out["samples"] = {k: v for k, v in sorted(ex.items())}
    print(json.dumps(out))


if __name__ == "__main__":
    main()
