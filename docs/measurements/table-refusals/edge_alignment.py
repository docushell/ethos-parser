import json, subprocess, collections, os
from pathlib import Path
B = Path(os.path.expanduser("~/ethos-external-benchmarks/opendataloader-bench"))
E = "./target/release/ethos-parser"
gt = json.loads((B/"ground-truth/reference.json").read_text())
FRAG = 60   # runs whose gap to the previous run's end is under this are one fragment chain

def fold(vs, tol):
    out=[]
    for v in sorted(vs):
        if not out or v-out[-1] > tol: out.append(v)
    return out
def line_of(lines, v):
    best=0
    for i,l in enumerate(lines):
        if l<=v: best=i
        else: break
    return best

def merged_spans(runs, rtol=150):
    """runs: [(x,y,adv)] -> [(row_index, left, right)] with mid-word fragments joined."""
    rows=fold([y for _,y,_ in runs], rtol)
    bylines=collections.defaultdict(list)
    for x,y,adv in runs:
        bylines[line_of(rows,y)].append((x, adv or 0))
    out=[]
    for ri, items in bylines.items():
        items.sort()
        left, right = items[0]
        right = left + right
        for x, adv in items[1:]:
            if x - right <= FRAG:              # a fragment of the same span
                right = max(right, x + adv)
            else:
                out.append((ri, left, right)); left, right = x, x + adv
        out.append((ri, left, right))
    return out, len(rows)

def support(spans, nrows, tol=150):
    """Strong alignment positions, counting LEFT and RIGHT edges separately."""
    if nrows < 3 or len(spans) < 6: return None
    best = 0
    for edge_ix in (1, 2):                      # 1 = left edges, 2 = right edges
        lines = fold([s[edge_ix] for s in spans], tol)
        rowsof = collections.defaultdict(set)
        for s in spans:
            rowsof[line_of(lines, s[edge_ix])].add(s[0])
        strong = sum(1 for rs in rowsof.values() if len(rs) >= nrows / 2)
        best = max(best, strong)
    return best

inside, outside = [], []
for doc,v in sorted(gt.items()):
    pdf=B/"pdfs"/doc
    if not pdf.exists(): continue
    r=subprocess.run([E,"extract",str(pdf)],capture_output=True)
    if r.returncode: continue
    rep=json.loads(r.stdout)["representation"]
    pages={i+1:p for i,p in enumerate(rep.get("pages",[]))}
    tb=collections.defaultdict(list)
    for e in v.get("elements",[]):
        if e.get("category")!="Table": continue
        pg=pages.get(e["page"])
        if not pg: continue
        xs=[c["x"]*pg["width"] for c in e["coordinates"]]; ys=[c["y"]*pg["height"] for c in e["coordinates"]]
        tb[e["page"]].append((min(xs),min(ys),max(xs),max(ys)))
    groups=collections.defaultdict(list)
    for n in rep["nodes"]:
        if n.get("kind")!="text_run": continue
        loc=(n.get("native_locator") or {}).get("pdf"); a=(n.get("attributes") or {}).get("text_run") or {}
        if not loc: continue
        x,y,p,adv=loc["origin_x"],loc["origin_y"],loc["page"],loc.get("advance")
        if any(x0<=x<=x1 and y0<=y<=y1 for x0,y0,x1,y1 in tb.get(p,[])):
            groups[("T",p)].append((x,y,adv))
        elif a.get("block") is not None:
            groups[("P",p,a["block"])].append((x,y,adv))
    for k,runs in groups.items():
        spans,nrows = merged_spans(runs)
        s = support(spans,nrows)
        if s is not None: (inside if k[0]=="T" else outside).append(s)

def q(a,p): a=sorted(a); return a[min(len(a)-1,int(len(a)*p))]
print(f"{'population':<26}{'n':>5}{'strong aligned cols: median':>30}{'p75':>7}")
for name,pool in (("INSIDE a GT table box",inside),("prose block",outside)):
    print(f"{name:<26}{len(pool):>5}{q(pool,.5):>30}{q(pool,.75):>7}")
print("\nSeparation — fraction above a threshold, with precision:")
for thr in (2,3,4,5,6):
    ti=sum(1 for s in inside if s>=thr); po=sum(1 for s in outside if s>=thr)
    prec=ti/(ti+po) if (ti+po) else 0
    print(f"  >={thr}: {ti}/{len(inside)} tables ({ti/max(len(inside),1):.0%}), "
          f"{po}/{len(outside)} prose ({po/max(len(outside),1):.0%})  -> precision {prec:.0%}")
