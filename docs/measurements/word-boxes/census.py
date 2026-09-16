"""Run census of representations: run shapes, whitespace, and visible runs with a zero advance.
Usage: census.py REPRESENTATION.json ..."""
import json, sys, collections, re

T = collections.Counter()
WS = re.compile(r"\s")
for path in sys.argv[1:]:
    d = json.load(open(path))
    r = d["representation"]
    geom = {g["node"]: g["presence"] for g in d["geometry"]}
    pages = {p["id"]: p for p in r["pages"]}
    C = collections.Counter()
    C["pages"] = len(pages)
    C["rotated_pages"] = sum(1 for p in pages.values() if p.get("rotation", 0) != 0)
    zero_pages = set()
    for n in r["nodes"]:
        if n["kind"] != "text_run":
            continue
        t = n["text"]
        a = n["attributes"]["text_run"]
        loc = n["native_locator"]["pdf"]
        pres = geom[n["id"]]
        C["runs"] += 1
        if len(t) == 1:
            C["single_char_runs"] += 1
        s = t.strip()
        if s and WS.search(s):
            C["runs_with_interior_whitespace"] += 1
            if pres["state"] == "measured":
                C["measured_runs_with_interior_whitespace"] += 1
        words = t.split()
        if not words:
            C["ws_only_runs"] += 1
        elif len(words) == 1:
            C["single_word_runs"] += 1
        else:
            C["multi_word_runs"] += 1
        nonws = sum(1 for ch in t if not ch.isspace())
        C["nonws_chars"] += nonws
        if len(words) > 1:
            C["nonws_chars_multi_word"] += nonws
        synth = {s_["char_index"] for s_ in a["synthesized"]}
        C["synth_chars"] += len(synth)
        codes = a["char_codes"]
        if len(t) == len(codes) + len(synth) and all(i >= len(codes) for i in synth):
            for i, ch in enumerate(t):
                if i in synth:
                    continue
                if ch.isspace():
                    if codes[i] == 32:
                        C["ws_from_code_32"] += 1
                    else:
                        C["ws_from_other_code"] += 1
        else:
            C["runs_codes_not_aligned_even_with_synth"] += 1
        if len(t) != len(codes):
            C["len_text_ne_len_codes"] += 1
            if synth and all(i >= len(codes) for i in synth) and len(t) - len(synth) == len(codes):
                C["mismatch_explained_by_trailing_synth"] += 1
        if a["scalar_code_mismatch"]:
            C["scalar_code_mismatch_true"] += 1
        adv = loc["advance"]
        if adv is None:
            C["advance_none"] += 1
        elif adv < 0:
            C["advance_negative"] += 1
        elif adv == 0:
            C["advance_zero"] += 1
            if s:
                C["advance_zero_visible_runs"] += 1
                C["advance_zero_visible_" + pres["state"] + "_" + str(pres.get("value") if pres["state"] == "absent" else "")] += 1
                C["nonws_chars_in_zero_advance_runs"] += nonws
                zero_pages.add(loc["page"])
        if pres["state"] == "measured" and adv is not None:
            x0, y0, x1, y1 = pres["value"]
            C["measured_runs"] += 1
            if abs(x0 - loc["origin_x"]) <= 1 and abs(x1 - (loc["origin_x"] + adv)) <= 1:
                C["measured_x_matches_origin_advance_1cpt"] += 1
            else:
                C["measured_x_mismatch"] += 1
    C["pages_with_zero_advance_visible_text"] = len(zero_pages)
    print(path, dict(sorted(C.items())))
    T.update(C)
print("TOTAL", dict(sorted(T.items())))
