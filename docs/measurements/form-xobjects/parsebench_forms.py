import pikepdf,glob,os,re,collections,json,sys
T=re.compile(rb"\b(Tj|TJ|'|\")\s")
def walk(xo,depth,seen,st):
    for name,x in (xo or {}).items():
        try:
            if x.get('/Subtype')!='/Form': continue
        except Exception: continue
        key=x.objgen
        st['draw_sites']+=1
        if key in seen: st['shared']+=1; continue
        seen.add(key); st['forms']+=1; st['depth']=max(st['depth'],depth)
        try: data=x.read_bytes()
        except Exception: st['undecodable']+=1; continue
        n=len(T.findall(data)); st['text_ops']+=n
        if n: st['text_forms']+=1
        if b'BDC' in data or b'BMC' in data: st['marked']+=1
        if b'/MCID' in data: st['mcid']+=1
        m=x.get('/Matrix'); 
        if m is not None and [float(v) for v in m]!=[1,0,0,1,0,0]: st['matrix']+=1
        res=x.get('/Resources')
        if res is None: st['no_resources']+=1
        elif '/Font' in res: st['own_fonts']+=1
        if '/StructParents' in x or '/StructParent' in x: st['structparents']+=1
        if '/Group' in x: st['group']+=1
        if res is not None: walk(res.get('/XObject'),depth+1,seen,st)
tot=collections.Counter(); rows=[]
for f in sorted(glob.glob(sys.argv[1])):
    try: pdf=pikepdf.open(f)
    except Exception: continue
    st=collections.Counter(); seen=set(); page_text=0
    for pg in pdf.pages:
        res=pg.get('/Resources')
        if res is not None: walk(res.get('/XObject'),1,seen,st)
        try:
            c=pg.get('/Contents'); data=b''.join(s.read_bytes() for s in c) if isinstance(c,pikepdf.Array) else (c.read_bytes() if c is not None else b'')
            page_text+=len(T.findall(data))
        except Exception: pass
    if st['text_forms']:
        st['docs']=1; st['tagged']=int('/StructTreeRoot' in pdf.Root); st['whole_page']=int(page_text==0)
        d=st.pop('depth'); tot['maxdepth']=max(tot['maxdepth'],d)
        tot.update(st); rows.append((os.path.basename(f)[:-4],st['text_ops'],page_text,d))
print(dict(tot))
for r in sorted(rows,key=lambda r:-r[1])[:12]: print('  ',r)
