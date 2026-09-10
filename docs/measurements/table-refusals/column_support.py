import json, subprocess, collections, os
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

def support(runs, ctol=150, rtol=150):
    """How well do column positions REPEAT across rows?

    A table has a few x-positions each appearing on most rows. Prose has many
    x-positions each appearing on one row.
    """
    if len(runs) < 6: return None
    cols=fold([x for x,_ in runs], ctol); rows=fold([y for _,y in runs], rtol)
    if len(cols)<2 or len(rows)<3: return None
    rowsof=collections.defaultdict(set)
    for x,y in runs:
        rowsof[line_of(cols,x)].add(line_of(rows,y))
    nrows=len(rows)
    # columns supported by at least half the rows
    strong=sum(1 for c,rs in rowsof.items() if len(rs) >= nrows/2)
    # mean support as a fraction of rows
    mean_support=sum(len(rs) for rs in rowsof.values())/len(rowsof)/nrows
    return strong, mean_support, len(cols), nrows

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
        x,y,p=loc["origin_x"],loc["origin_y"],loc["page"]
        if any(x0<=x<=x1 and y0<=y<=y1 for x0,y0,x1,y1 in tb.get(p,[])):
            groups[("T",p)].append((x,y))
        elif a.get("block") is not None:
            groups[("P",p,a["block"])].append((x,y))
    for k,runs in groups.items():
        s=support(runs)
        if s: (inside if k[0]=="T" else outside).append(s)

def q(a,p): a=sorted(a); return a[min(len(a)-1,int(len(a)*p))]
print(f"{'population':<26}{'n':>5}{'strong cols median':>20}{'mean support median':>21}")
for name,pool in (("INSIDE a GT table box",inside),("prose block",outside)):
    print(f"{name:<26}{len(pool):>5}{q([r[0] for r in pool],.5):>20}{q([r[1] for r in pool],.5):>20.0%}")
print()
print("Separation check — fraction of each population above a 'strong columns' threshold:")
for thr in (2,3,4,5):
    ti=sum(1 for r in inside if r[0]>=thr); po=sum(1 for r in outside if r[0]>=thr)
    prec = ti/(ti+po) if (ti+po) else 0
    print(f"  >={thr} strong cols: {ti}/{len(inside)} tables ({ti/len(inside):.0%}), "
          f"{po}/{len(outside)} prose ({po/len(outside):.0%})  -> precision {prec:.0%}")
