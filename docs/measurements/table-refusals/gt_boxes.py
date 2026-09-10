import json, subprocess, collections, os, statistics
from pathlib import Path
B = Path(os.path.expanduser("~/ethos-external-benchmarks/opendataloader-bench"))
E = "./target/release/ethos-parser"
gt = json.loads((B/"ground-truth/reference.json").read_text())

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

def lattice(runs, ctol, rtol):
    """runs: [(x,y)] -> (cols, rows, faces, occupancy)"""
    if len(runs)<4: return None
    cols=fold([x for x,_ in runs], ctol); rows=fold([y for _,y in runs], rtol)
    if len(cols)<2 or len(rows)<2: return None
    faces=len(cols)*len(rows)
    occ={(line_of(rows,y),line_of(cols,x)) for x,y in runs}
    return len(cols), len(rows), faces, len(occ)/faces

inside, outside = [], []
for doc, v in sorted(gt.items()):
    pdf=B/"pdfs"/doc
    if not pdf.exists(): continue
    r=subprocess.run([E,"extract",str(pdf)],capture_output=True)
    if r.returncode: continue
    rep=json.loads(r.stdout)["representation"]
    pages={p["number"] if "number" in p else i+1: p for i,p in enumerate(rep.get("pages",[]))}
    tboxes=collections.defaultdict(list)
    for e in v.get("elements",[]):
        if e.get("category")!="Table": continue
        pg=pages.get(e["page"])
        if not pg: continue
        xs=[c["x"]*pg["width"] for c in e["coordinates"]]; ys=[c["y"]*pg["height"] for c in e["coordinates"]]
        tboxes[e["page"]].append((min(xs),min(ys),max(xs),max(ys)))
    byblock=collections.defaultdict(list)
    for n in rep["nodes"]:
        if n.get("kind")!="text_run": continue
        loc=(n.get("native_locator") or {}).get("pdf"); a=(n.get("attributes") or {}).get("text_run") or {}
        if not loc: continue
        x,y,p=loc["origin_x"],loc["origin_y"],loc["page"]
        hit=any(x0<=x<=x1 and y0<=y<=y1 for x0,y0,x1,y1 in tboxes.get(p,[]))
        if hit:
            byblock[("T",p,tuple(tboxes[p]))].append((x,y))
        elif a.get("block") is not None:
            byblock[("P",p,a["block"])].append((x,y))
    for key, runs in byblock.items():
        for ctol,rtol,label in ((150,150,"tight"),(1200,600,"floor")):
            L=lattice(runs,ctol,rtol)
            if not L: continue
            (inside if key[0]=="T" else outside).append((label,)+L)

for label in ("tight","floor"):
    print("\n"+"="*72); print(f"fold tolerance: {label}"); print("="*72)
    for name, pool in (("INSIDE a GT table box", inside), ("prose block", outside)):
        v=[r for r in pool if r[0]==label]
        if not v: continue
        cols=sorted(r[1] for r in v); occ=sorted(r[4] for r in v)
        q=lambda a,p: a[min(len(a)-1,int(len(a)*p))]
        print(f"{name:<24} n={len(v):<5} cols median={q(cols,.5):<5} "
              f"occupancy p25={q(occ,.25):.0%} median={q(occ,.5):.0%} p75={q(occ,.75):.0%} max={occ[-1]:.0%}")
