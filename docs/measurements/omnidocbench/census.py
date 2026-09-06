"""Census of ethos-parser over OmniDocBench's `v1_0` `ori_pdfs` (981 single-page PDFs).

    python3 docs/measurements/omnidocbench/census.py <dir-of-pdfs>

The instrument behind the 0.47.0 CHANGELOG entry and `README.md` beside this file, committed so
those numbers can be re-derived rather than believed — the same correction
`../block-subdivision/` and `../opendataloader-bench/` make for their own measurements.

**This is a measurement script, not product code.** It is not on the gate, nothing imports it, and
it reads artifacts this engine emits rather than reaching into it.

**It computes nothing the engine does not say**, with one exception: the block-length statistics,
which are a property of the emitted Markdown string and are named as such below.

**The corpus is not vendored and must not be.** OmniDocBench's dataset card states the data is for
research use only, and the precedent beside this file commits the instrument and never the corpus.
Fetch it yourself; the branch and commit it came from are in `README.md`.
"""
import json,subprocess,glob,os,re,statistics,sys,tempfile,time
from concurrent.futures import ThreadPoolExecutor

B=os.environ.get("ETHOS_PARSER_BIN","target/release/ethos-parser")

def one(f):
    r={'file':os.path.basename(f),'fam':os.path.basename(f).split('_')[0],'bytes':os.path.getsize(f)}
    t0=time.time()
    p=subprocess.run([B,'classify',f],capture_output=True)
    r['classify_exit']=p.returncode
    if p.returncode in (0,1):
        try:
            c=json.loads(p.stdout)
            r.update(pages=c.get('page_count'),pages_with_text=c.get('pages_with_text'),
                     ocr=c.get('ocr_reasons',[]),layout=c.get('layout_reasons',[]),
                     text_bytes=sum(x.get('text_bytes',0) for x in c.get('pages',[])),
                     images=sum(x.get('image_count',0) for x in c.get('pages',[])),
                     annots=sum(x.get('annotations',0) for x in c.get('pages',[])))
        except Exception as e: r['classify_parse_err']=str(e)[:200]
    else:
        r['classify_err']=p.stderr.decode(errors='replace').strip()[:400]

    p=subprocess.run([B,'extract',f],capture_output=True)
    r['extract_exit']=p.returncode
    if p.returncode==0:
        rep=json.loads(p.stdout)['representation']
        a=rep.get('assurance',{})
        r['lims']=sorted({l['code'] for l in a.get('limitations',[])})
        r['nodes']=len(rep.get('nodes',[]))
        r['tables']=len(rep.get('tables',[]) or [])
        r['coverage']=a.get('coverage',{})
        with tempfile.NamedTemporaryFile(suffix='.json',delete=False) as fh: fh.write(p.stdout); tmp=fh.name
        m=subprocess.run([B,'markdown',tmp],capture_output=True); os.unlink(tmp)
        r['md_exit']=m.returncode
        if m.returncode==0:
            md=json.loads(m.stdout); s=md.get('markdown','')
            r['md_len']=len(s); r['md_head']=s[:200]
            cov=md.get('coverage',{})
            r['dropped']={x.get('code'):x.get('nodes') for x in cov.get('dropped',[])}
            r['erasures']=cov.get('structural_erasures')
            r['pipes']=s.count('|')
            blocks=[b.strip() for b in re.split(r'\n\s*\n',s) if b.strip()]
            if blocks:
                lens=[len(b) for b in blocks]
                r['blocks']=len(blocks); r['median_block']=statistics.median(lens)
                r['mean_block']=round(statistics.fmean(lens),2)
                r['pct_tiny']=round(sum(1 for l in lens if l<=2)/len(lens),4)
            else: r['blocks']=0
            r['cjk']=sum(1 for ch in s if '一'<=ch<='鿿')
        else: r['md_len']=0; r['md_err']=m.stderr.decode(errors='replace').strip()[:300]
    else:
        r['extract_err']=p.stderr.decode(errors='replace').strip()[:400]
        r['lims']=[]; r['nodes']=0; r['md_len']=0
    r['ms']=round((time.time()-t0)*1000)
    return r

files=sorted(glob.glob(os.path.join(sys.argv[1] if len(sys.argv)>1 else "pdfs","*.pdf")))
t0=time.time()
with ThreadPoolExecutor(8) as ex: rows=list(ex.map(one,files))
json.dump(rows,open('full_census.json','w'),indent=1)
print(f"{len(rows)} documents in {time.time()-t0:.0f}s wall")
