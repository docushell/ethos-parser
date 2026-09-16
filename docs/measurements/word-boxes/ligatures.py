"""Runs whose codes do not map one-to-one onto the characters of their text, beyond a trailing
synthesized space, whether they carry a measured box, and whether that box is the run's own advance
wide — a code advances the pen once however many characters it decodes to, so a ligature widens the
box by one glyph and not by its letters (`docs/01-CONTRACT.md` §5.3). Several representations print
one total, each loaded whole in turn. Usage: ligatures.py REPRESENTATION.json ..."""
import json, sys, collections
c = collections.Counter(); texts = collections.Counter(); width = collections.Counter(); docs = set()
for path in sys.argv[1:]:
    d = json.load(open(path))
    geom = {g["node"]: g["presence"] for g in d["geometry"]}
    for n in d["representation"]["nodes"]:
        if n["kind"] != "text_run": continue
        a = n["attributes"]["text_run"]
        if not a["scalar_code_mismatch"]: continue
        c["flagged"] += 1
        synth = {s["char_index"] for s in a["synthesized"]}
        real = len([ch for i, ch in enumerate(n["text"]) if i not in synth])
        if real == len(a["char_codes"]):
            c["explained_by_synthesized_chars"] += 1
            continue
        pres = geom[n["id"]]
        c["codes_decode_to_several_chars", pres["state"]] += 1
        texts[n["text"]] += 1; docs.add(path)
        if pres["state"] != "measured": continue
        # Box and advance are both integer centipoints, and the box's two edges quantize
        # separately, so one centipoint apart is the same measurement.
        box = pres["value"][2] - pres["value"][0]; adv = n["native_locator"]["pdf"].get("advance")
        if adv is None: width["advance_absent"] += 1
        elif box == adv: width["box_width_is_the_advance"] += 1
        elif abs(box - adv) <= 1: width["within_1_centipoint"] += 1
        else: width["other", n["text"][:16], box, adv] += 1
print(dict(c)); print("documents with a multi-character code:", len(docs))
print(dict(width)); print(texts.most_common(8))
