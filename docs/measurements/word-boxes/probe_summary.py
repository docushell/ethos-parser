"""Print each probe representation's text runs: codes, advance, origin and geometry.
Usage: probe_summary.py pNN-*.json ..."""
import json,glob,sys
for f in sorted(glob.glob('p1[3-9]*.json')+glob.glob('p2*.json')) if len(sys.argv)<2 else sys.argv[1:]:
    try: d=json.load(open(f))
    except Exception as e: print(f,'ERR',open(f.replace('.json','.err')).read()[:300]); continue
    g={x['node']:x['presence'] for x in d['geometry']}
    pg=d['representation']['pages']
    for n in d['representation']['nodes']:
        if n['kind']!='text_run': continue
        a=n['attributes']['text_run']; l=n['native_locator']['pdf']
        print(f, repr(n['text']), 'codes',a['char_codes'], 'scm',a['scalar_code_mismatch'], 'synth',[s['char_index'] for s in a['synthesized']], 'adv',l.get('advance'),'o',l['origin_x'],l['origin_y'], g[n['id']], 'page', pg[0]['width'],pg[0]['height'],pg[0].get('rotation'))
