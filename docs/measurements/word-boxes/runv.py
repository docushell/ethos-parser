"""Verify nine claims against each grounding artifact with the pinned Ethos binary, and print what
each resolves to. Usage: ETHOS_BIN=../ethos-oracle/target/release/ethos runv.py ARTIFACT.ground.json ..."""
import json, hashlib, os, subprocess, sys
E = os.environ["ETHOS_BIN"]
claims = [
 ("c1 quote run-contained multiword span_id s25", {"kind":"quote","text":"An entry is required.","citation":{"page":"p1","span_id":"s25"}}),
 ("c2 multiword quote on one word span s25.w3", {"kind":"quote","text":"An entry is required.","citation":{"page":"p1","span_id":"s25.w3"}}),
 ("c3 single word span_id s25.w6", {"kind":"quote","text":"required.","citation":{"page":"p1","span_id":"s25.w6"}}),
 ("c4 page+bbox of word s25.w6", {"kind":"quote","text":"required.","citation":{"page":"p1","bbox":[19580,9734,22308,10550]}}),
 ("c5 element_id e8 partial", {"kind":"quote","text":"required.","citation":{"page":"p1","element_id":"e8"}}),
 ("c6 page-only quote", {"kind":"quote","text":"required.","citation":{"page":"p1"}}),
 ("c7 page-only value 'required.'", {"kind":"value","text":"required.","citation":{"page":"p1"}}),
 ("c8 span_id s6 'Treasury'", {"kind":"quote","text":"Treasury","citation":{"page":"p1","span_id":"s6"}}),
 ("c9 two-word quote across two word spans cited by one", {"kind":"quote","text":"is required.","citation":{"page":"p1","span_id":"s25.w5"}}),
]
for art in sys.argv[1:]:
    b = open(art,'rb').read()
    fp = "sha256:" + hashlib.sha256(b).hexdigest()
    body = {"document_fingerprint": fp, "claims":[c for _,c in claims]}
    cp = art + ".claims.json"
    json.dump(body, open(cp,'w'))
    r = subprocess.run([E,"verify",art,"--citations",cp,"--grounding","ethos-grounding-json"],capture_output=True,text=True)
    try:
        rep = json.loads(r.stdout)
    except Exception:
        print(art, "exit", r.returncode, r.stdout[:300], r.stderr[:300]); continue
    p = rep.get("predicate", rep)
    print("==", art.split('/')[-1], "exit", r.returncode, "warnings", p.get("warnings"), "capability_limits", p.get("capability_limits"), "proof", {k:v for k,v in p.items() if 'proof' in k})
    checks = p.get("checks") or p.get("results")
    for (name,_), ch in zip(claims, checks):
        ev = ch.get("evidence") or {}
        print("  ", name, "|", ch.get("status"), ch.get("reason"), ch.get("match_method") or ch.get("method"), ch.get("evidence_tier") or ch.get("tier"), ev.get("bbox"), repr((ev.get("text") or "")[:40]))
