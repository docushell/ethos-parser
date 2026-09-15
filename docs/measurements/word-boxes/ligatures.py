"""Runs whose codes do not map one-to-one onto the characters of their text, beyond a trailing
synthesized space, and whether they carry a measured box. Usage: ligatures.py REPRESENTATION.json"""
import json, sys, collections
d = json.load(open(sys.argv[1]))
state = {g["node"]: g["presence"]["state"] for g in d["geometry"]}
c = collections.Counter(); texts = collections.Counter()
for n in d["representation"]["nodes"]:
    if n["kind"] != "text_run": continue
    a = n["attributes"]["text_run"]
    if not a["scalar_code_mismatch"]: continue
    synth = {s["char_index"] for s in a["synthesized"]}
    real = len([ch for i, ch in enumerate(n["text"]) if i not in synth])
    if real == len(a["char_codes"]):
        c["explained_by_synthesized_chars"] += 1
    else:
        c["codes_decode_to_several_chars", state[n["id"]]] += 1
        texts[n["text"]] += 1
print(dict(c)); print(texts.most_common(8))
