#!/usr/bin/env python3
"""Categorise a kept differential directory for the rotated-text change (docs/22 §9 items 1-2).

usage: rotcheck.py DIR [--no-ground]

DIR holds run.py's extract.a/.b (and ground/markdown/html .a/.b when they exist). The extracts are
STREAMED — geometry and nodes are walked element by element in lockstep — so a 950 MiB extract
never sits in memory; only the nodes whose geometry changed are remembered.

Prints one JSON object:
  categories      every node-level difference (as reprdiff.py names them), plus
                  A:no_ink->measured:{adv<=0|adv>0}:{vertical|horizontal|other}
                  B:measured->measured:{vertical_signature|other}
                  C:->not_axis_aligned, D:->measured_off_page:{flagged|unflagged}
  other_keys      top-level / representation keys that differ, other than nodes and geometry
  limitations     codes added/removed, and for geometry-absent-not-groundable which clauses appear
  markdown/html   equal after normalising representation_sha256
  ground          id-insensitive: spans keyed by node id, elements keyed by member span ids
"""
import hashlib
import io
import json
import os
import re
import sys
from collections import Counter, defaultdict

CHUNK = 8 << 20


class Stream:
    def __init__(self, path):
        self.f = io.open(path, "r", encoding="utf-8")
        self.buf = ""
        self.pos = 0
        self.eof = False
        self.dec = json.JSONDecoder()

    def fill(self):
        if self.eof:
            return False
        chunk = self.f.read(CHUNK)
        if not chunk:
            self.eof = True
            return False
        self.buf = self.buf[self.pos:] + chunk
        self.pos = 0
        return True

    def peek(self):
        while self.pos >= len(self.buf):
            if not self.fill():
                return ""
        return self.buf[self.pos]

    def expect(self, ch):
        c = self.peek()
        if c != ch:
            raise ValueError("expected %r at %d, got %r" % (ch, self.pos, c))
        self.pos += 1

    def value(self):
        while True:
            try:
                v, end = self.dec.raw_decode(self.buf, self.pos)
                # A number at the end of the buffer may be truncated; strings/objects cannot parse
                # truncated, but a number can. Refill if the value touches the end.
                if end >= len(self.buf) and not self.eof:
                    raise json.JSONDecodeError("edge", self.buf, end)
                self.pos = end
                return v
            except json.JSONDecodeError:
                if not self.fill():
                    v, end = self.dec.raw_decode(self.buf, self.pos)
                    self.pos = end
                    return v

    def key(self):
        k = self.value()
        self.expect(":")
        return k

    def obj_keys(self):
        """Iterate the keys of the object at the cursor; the caller consumes each value."""
        self.expect("{")
        first = True
        while True:
            c = self.peek()
            if c == "}":
                self.pos += 1
                return
            if not first:
                self.expect(",")
            first = False
            yield self.key()

    def array_items(self):
        self.expect("[")
        first = True
        while True:
            c = self.peek()
            if c == "]":
                self.pos += 1
                return
            if not first:
                self.expect(",")
            first = False
            yield self.value()


def presence_key(p):
    if p is None:
        return "none"
    if p["state"] == "measured":
        return "measured"
    return "absent:" + str(p.get("value"))


def walk(path, on_geometry, on_nodes, other):
    """Walk one extract, handing geometry and node arrays to callbacks as generators."""
    s = Stream(path)
    for k in s.obj_keys():
        if k == "geometry":
            on_geometry(s.array_items())
        elif k == "representation":
            for rk in s.obj_keys():
                if rk == "nodes":
                    on_nodes(s.array_items())
                else:
                    other["representation." + rk] = s.value()
        else:
            other[k] = s.value()


def lockstep(path_a, path_b):
    """Two extracts walked together. Generators cannot be walked in lockstep across two
    independent parses without threads, so each file is walked once per array into a small
    record of what changed: first geometry (keeping only differing entries), then nodes."""
    other_a, other_b = {}, {}

    # Pass 1 over A and B geometry: store A geometry compactly as a digest per index, and then
    # compare B against it. Geometry entries are tiny, so a per-index hash of 8 bytes is ~16 MiB
    # for two million runs.
    hashes = []

    def geo_a_cb(items):
        for g in items:
            hashes.append((g["node"], hashlib.blake2b(json.dumps(g["presence"], sort_keys=True).encode(), digest_size=8).digest()))

    diffs = {}

    def geo_b_cb(items):
        count = 0
        for i, g in enumerate(items):
            count += 1
            if i >= len(hashes):
                diffs[i] = (None, g["node"], None, g["presence"])
                continue
            node, h = hashes[i]
            hb = hashlib.blake2b(json.dumps(g["presence"], sort_keys=True).encode(), digest_size=8).digest()
            if node != g["node"] or h != hb:
                diffs[i] = (node, g["node"], None, g["presence"])
        for j in range(count, len(hashes)):
            diffs[j] = (hashes[j][0], None, None, None)

    walk(path_a, geo_a_cb, lambda it: sum(1 for _ in it), {})
    walk(path_b, geo_b_cb, lambda it: sum(1 for _ in it), {})
    n_geo = len(hashes)
    hashes.clear()

    # Pass 2: recover A's presence for the differing indices.
    def geo_a_fill(items):
        for i, g in enumerate(items):
            if i in diffs:
                na, nb, _, pb = diffs[i]
                diffs[i] = (na, nb, g["presence"], pb)

    # Nodes: A's node records only for ids in the diff set, and a digest of every node so B can be
    # compared field by field only where the digest differs.
    node_digest = []
    nodes_a_keep = {}
    diff_ids = {v[0] for v in diffs.values() if v[0]} | {v[1] for v in diffs.values() if v[1]}

    def nodes_a_cb(items):
        for n in items:
            node_digest.append((n["id"], hashlib.blake2b(json.dumps(n, sort_keys=True).encode(), digest_size=8).digest()))
            if n["id"] in diff_ids:
                nodes_a_keep[n["id"]] = n

    walk(path_a, geo_a_fill, nodes_a_cb, other_a)

    cats = Counter()
    samples = defaultdict(list)

    def hit(cat, nid):
        cats[cat] += 1
        if len(samples[cat]) < 3:
            samples[cat].append(nid)

    changed_nodes_b = {}
    nodes_needing_a = {}

    def nodes_b_cb(items):
        count = 0
        for i, n in enumerate(items):
            count += 1
            if i >= len(node_digest):
                hit("node_added", n["id"])
                continue
            ida, ha = node_digest[i]
            hb = hashlib.blake2b(json.dumps(n, sort_keys=True).encode(), digest_size=8).digest()
            if ida != n["id"]:
                hit("node_misaligned", n["id"])
            if ha != hb:
                nodes_needing_a[i] = n
            if n["id"] in diff_ids:
                changed_nodes_b[n["id"]] = n
        for j in range(count, len(node_digest)):
            hit("node_removed", node_digest[j][0])

    walk(path_b, lambda it: sum(1 for _ in it), nodes_b_cb, other_b)
    n_nodes = len(node_digest)

    # Field-level node diffs need A's full record at those indices (a third, cheap pass).
    if nodes_needing_a:
        full_a = {}

        def nodes_a_full(items):
            for i, n in enumerate(items):
                if i in nodes_needing_a:
                    full_a[i] = n

        walk(path_a, lambda it: sum(1 for _ in it), nodes_a_full, {})
        for i, y in nodes_needing_a.items():
            x = full_a[i]
            nid = y["id"]
            for f in ("kind", "text", "parent", "ordinal", "derivation", "structural_locator"):
                if x.get(f) != y.get(f):
                    hit(f, nid)
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
            extra = set(x) ^ set(y) | {k for k in set(x) & set(y) if k not in ("kind", "text", "parent", "ordinal", "derivation", "structural_locator", "attributes", "native_locator", "id") and x[k] != y[k]}
            for k in sorted(extra):
                hit("node_field:" + k, nid)

    # Geometry transitions, with the rotation signatures.
    for i, (na, nb, pa, pb) in sorted(diffs.items()):
        nid = nb or na
        if na != nb:
            hit("geometry_misaligned", nid)
            continue
        ka, kb = presence_key(pa), presence_key(pb)
        node = changed_nodes_b.get(nid) or nodes_a_keep.get(nid)
        loc = (node or {}).get("native_locator", {}).get("pdf", {})
        ox, oy, adv = loc.get("origin_x"), loc.get("origin_y"), loc.get("advance")
        findings = ((node or {}).get("attributes", {}).get("text_run") or {}).get("findings", [])
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
            sig = "vertical_signature" if (vb[0] < ox < vb[2] and oy in (vb[1], vb[3])) else "other"
            hit("B:measured->measured:" + sig, nid)
        else:
            hit("geom:%s->%s" % (ka, kb), nid)
            if kb == "measured":
                vb = pb["value"]
                if vb[0] < ox < vb[2] and oy in (vb[1], vb[3]):
                    orient = "vertical"
                elif vb[1] < oy < vb[3] and ox in (vb[0], vb[2]):
                    orient = "horizontal"
                else:
                    orient = "other"
                hit("A:%s->measured:%s:%s" % (ka, "adv<=0" if adv is not None and adv <= 0 else "adv>0", orient), nid)
            elif kb == "absent:not_axis_aligned":
                hit("C:%s->not_axis_aligned" % ka, nid)
            elif kb == "absent:measured_off_page":
                hit("D:%s->measured_off_page:%s" % (ka, "flagged" if "off_page" in findings or "off-page" in findings else "unflagged"), nid)

    out = {"nodes": n_nodes, "geometry": n_geo}
    other_keys = sorted(k for k in set(other_a) | set(other_b) if other_a.get(k) != other_b.get(k))
    out["other_keys"] = other_keys
    la = other_a.get("representation.assurance", {}).get("limitations", [])
    lb = other_b.get("representation.assurance", {}).get("limitations", [])
    lim = {}
    ca = {l["code"]: l["detail"] for l in la}
    cb = {l["code"]: l["detail"] for l in lb}
    lim["added"] = sorted(set(cb) - set(ca))
    lim["removed"] = sorted(set(ca) - set(cb))
    lim["detail_changed"] = sorted(c for c in set(ca) & set(cb) if ca[c] != cb[c])
    g = "geometry-absent-not-groundable"
    for side, d in (("a", ca), ("b", cb)):
        t = d.get(g)
        if t is not None:
            lim[g + "." + side] = {
                "head": t.split(" text node(s)")[0],
                "reasons": re.search(r"\*\*(\w+) reasons", t).group(1) if re.search(r"\*\*(\w+) reasons", t) else None,
                "slices": re.search(r"reader\*\* \(([^)]*)\)", t).group(1) if re.search(r"reader\*\* \(([^)]*)\)", t) else None,
                "offpage_legacy_sentence": "its origin and an `off-page-text` finding" in t,
                "offpage_unflagged_branch": "have their origin inside the visible page" in t,
                "not_axis_aligned_clause": "do not run along an axis" in t,
            }
    # Assurance other than limitations.
    aa = dict(other_a.get("representation.assurance", {}))
    ab = dict(other_b.get("representation.assurance", {}))
    aa.pop("limitations", None)
    ab.pop("limitations", None)
    if aa != ab:
        out["other_keys"].append("assurance(other than limitations)")
    out["limitations"] = lim
    out["categories"] = dict(sorted(cats.items()))
    out["samples"] = dict(sorted(samples.items()))
    return out


def norm_sha(path):
    with open(path, "rb") as f:
        b = f.read()
    return re.sub(rb'"representation_sha256":"sha256:[0-9a-f]{64}"', b'"representation_sha256":"X"', b)


def ground_diff(pa, pb):
    a = json.load(open(pa, "rb"))
    b = json.load(open(pb, "rb"))
    out = {}
    for k in sorted(set(a) | set(b)):
        if k in ("elements", "spans"):
            continue
        if a.get(k) != b.get(k):
            out.setdefault("other_keys", []).append(k)
    sa = {s["id"]: s for s in (a.get("spans") or [])}
    sb = {s["id"]: s for s in (b.get("spans") or [])}
    out["spans"] = [len(sa), len(sb)]
    out["spans_added"] = len(sb.keys() - sa.keys())
    out["spans_removed"] = len(sa.keys() - sb.keys())
    strip = lambda s: {k: v for k, v in s.items() if k not in ("element", "bbox")}
    out["spans_bbox_changed"] = sum(1 for i in sa.keys() & sb.keys() if sa[i]["bbox"] != sb[i]["bbox"])
    out["spans_other_changed"] = sum(1 for i in sa.keys() & sb.keys() if strip(sa[i]) != strip(sb[i]))
    # Elements keyed by member spans (order-insensitive), which ignores renumbering.
    def members(doc):
        m = defaultdict(list)
        for s in doc.get("spans") or []:
            m[s["element"]].append(s["id"])
        return m
    ma, mb = members(a), members(b)
    ea = {}
    for e in a.get("elements") or []:
        ea[tuple(sorted(ma.get(e["id"], []))) or ("text:" + str(e.get("text")), e.get("page"), str(e.get("bbox")))] = e
    eb = {}
    for e in b.get("elements") or []:
        eb[tuple(sorted(mb.get(e["id"], []))) or ("text:" + str(e.get("text")), e.get("page"), str(e.get("bbox")))] = e
    out["elements"] = [len(a.get("elements") or []), len(b.get("elements") or [])]
    out["elements_added"] = len(eb.keys() - ea.keys())
    out["elements_removed"] = len(ea.keys() - eb.keys())
    common = ea.keys() & eb.keys()
    out["elements_bbox_changed"] = sum(1 for k in common if ea[k].get("bbox") != eb[k].get("bbox"))
    out["elements_text_changed"] = sum(1 for k in common if ea[k].get("text") != eb[k].get("text"))
    out["tables_equal"] = a.get("tables") == b.get("tables")
    return out


def main():
    d = sys.argv[1]
    res = {"dir": os.path.basename(d.rstrip("/"))}
    ea, eb = os.path.join(d, "extract.a"), os.path.join(d, "extract.b")
    if os.path.exists(ea) and os.path.exists(eb) and os.path.getsize(ea) and os.path.getsize(eb):
        try:
            res["extract"] = lockstep(ea, eb)
        except Exception as e:  # report, never hide
            res["extract_error"] = repr(e)
    for cmd in ("markdown", "html"):
        pa, pb = os.path.join(d, cmd + ".a"), os.path.join(d, cmd + ".b")
        if os.path.exists(pa) and os.path.exists(pb):
            res[cmd + "_equal_modulo_representation_sha256"] = norm_sha(pa) == norm_sha(pb)
    ga, gb = os.path.join(d, "ground.a"), os.path.join(d, "ground.b")
    if "--no-ground" not in sys.argv and os.path.exists(ga) and os.path.exists(gb) and os.path.getsize(ga) and os.path.getsize(gb):
        res["ground"] = ground_diff(ga, gb)
    print(json.dumps(res))


if __name__ == "__main__":
    main()
