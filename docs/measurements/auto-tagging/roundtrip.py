#!/usr/bin/env python3
"""The round trip of docs/23-AUTO-TAGGING-SCOPE.md §7.1 on every untagged document the repository
can reach, and the consumer demonstration of §7.2's first half.

usage: roundtrip.py --branch BIN --release BIN --ethos BIN --out DIR --results DIR
                    [--qpdf EXE] [--jobs 4] [--gate-timeout 300] [--commit SHA] LABEL=DIR ...

Each LABEL=DIR names a corpus: every `*.pdf` under DIR, recursively, in path order. The label
`gate` is special: those documents are tagged already, so they are not round-tripped; they get the
wire-cost measurement of scope §4.2 instead (one `extract` each with the branch binary, sequential,
time-boxed by --gate-timeout).

For every other document, with the branch binary:

1. `tag` — stdout to a file, exit code and stderr kept. A refusal is bucketed by the stable prefix
   of its message onto the rows of scope §3.6 (REFUSALS below); a message no row matches is kept
   under `other` with its digits masked.
2. On success: `qpdf --json` of the output, counting the `/StructElem` objects by `/S` and every
   `/Div`'s `/K` ids — elements written, sequences written, sequences per block. The writer holds
   each sequence's placement and split cause in its plan and prints neither, so §7.1's split-cause
   buckets cannot be counted from outside; only the distribution can.
3. `extract` of the tagged PDF and of the original; `ground`, `markdown` and `html` on both.
   Projection equality is byte equality of the canonical serialisation (`json.dumps` with
   `sort_keys=True`, `separators=(",", ":")`, `ensure_ascii=False`, UTF-8) after removing, on both
   sides: every `assurance` member (top level and under `representation`), `source_sha256` and
   `representation_sha256` from a Markdown or HTML artifact, and `source.sha256` from a grounding
   artifact — the members that name the input bytes and so differ on any tagged/original pair by
   construction. The first differing field, in sorted-key order, is recorded for a differing pair,
   and for Markdown and HTML whether the projected text itself differs.
4. `grounding-check --source-artifact <tagged.pdf>` on the tagged grounding artifact: exit code,
   `structure` and `source_binding`.
5. `ethos verify` on one claim quoting the full text of the first non-whitespace run of the first
   block of the tagged representation (the first `text_run` node bound `pdf_tagged`, in node
   order), cited at the first element on that run's page whose text contains it — looked up
   independently in the tagged and in the original grounding artifact, each claims file carrying
   that artifact's own sha256 as `document_fingerprint`. The two reports must agree on
   `predicate.all_evidence_grounded` and on the claim's `status`, `evidence_tier` and
   `match_method`.
6. The consumer demonstration: the RELEASE binary's `extract` on the tagged PDF, counting runs
   bound `pdf_tagged`, role paths equal to `Document/Div`, locators carrying a `derivation` key
   (a reader that knows the attribute writes one; 0.58.0 does not), and the limitation codes.

For every document, tagged or refused, the original is also extracted, so that documents whose
own artifact declares `mcid-property-list-by-name` or `inline-images-not-emitted` can be counted
by outcome; and its bytes are scanned for reals outside content streams (`stream … endstream`
data blanked first) inside `/MediaBox`, `/CropBox`, `/Rect`, `/BBox`, `/Matrix`, `/FontMatrix`
and `/Widths` arrays, counting the ones whose shortest `f32` representation is not the number the
source wrote (`shortest_f32`), by key. A `/Widths` array reached through an indirect reference and
a dictionary inside an object stream are not visible to a raw scan; documents carrying `/ObjStm`
are counted so the reach of the scan is stated. The `/Producer` string comes from `qpdf --json` of
the original.

Writes RESULTS/roundtrip.jsonl (a header line naming the commit and the binaries, then one line
per document) and RESULTS/roundtrip-summary.json, and prints the summary as Markdown tables.
Artifacts stay under OUT/<corpus>/<name>/ and are never committed. Stdlib only, Python 3.9.
"""
import argparse
import hashlib
import json
import os
import re
import struct
import subprocess
import sys
import time
from collections import Counter, defaultdict
from concurrent.futures import ProcessPoolExecutor
from decimal import Decimal, InvalidOperation

DERIVATION_KEY = b'"derivation":"extracted",'
PDF_TAGGED_KEY = b'"pdf_tagged":'
SEQUENTIAL_ABOVE_PAGES = 50

# (bucket, the row of scope §3.6 it counts, regex over the first `engine:` line of stderr)
REFUSALS = [
    ("tree-present", "row 1", r"the catalog declares /StructTreeRoot"),
    ("mcid-inline-no-tree", "row 2", r"carries (?:a )?marked-content id"),
    ("mcid-by-name", "row 3", r"names a property list that carries `/MCID`"),
    ("property-list-unresolvable", "row 3", r"names a property list (?:the page's `/Properties`|that resolves to)"),
    ("encrypted", "row 4", r"^engine: encrypted source"),
    ("filter", "row 5", r"^engine: unsupported content stream filter"),
    ("decode-to-end", "row 5", r"^engine: malformed content stream"),
    ("tokeniser-disagreement", "row 6", r"^engine: unsupported content stream tokeniser"),
    ("struct-parent-key", "§3.6, amended", r"carries /StructParents? and the catalog declares no /StructTreeRoot"),
    ("operator-in-two-blocks", "§3.6, amended", r"shows runs the cut placed in two blocks"),
    ("no-block-to-tag", "§3.4", r"no text run this engine reads outside an /Artifact frame"),
    ("page-not-processed", "no page budget", r"was not processed"),
    ("self-check", "row 7", r"^engine: malformed tagging self-check"),
]

KEY_ARRAY = re.compile(rb"/(MediaBox|CropBox|Rect|BBox|Matrix|FontMatrix|Widths)\s*\[([^\]]*)\]")
STREAM_DATA = re.compile(rb"stream\r?\n.*?endstream", re.S)
TOKEN = re.compile(rb"[^\s\[\]<>(){}/%]+")
REAL = re.compile(rb"^[+-]?(?:\d+\.\d*|\.\d+)$")


# --- process helpers ------------------------------------------------------------------------------

def run(argv, out=None, timeout=None):
    """Run argv; stdout to `out` (a path) or captured; returns (exit, stderr, stdout_bytes)."""
    if out is None:
        p = subprocess.run(argv, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=timeout)
        return p.returncode, p.stderr.decode(errors="replace"), p.stdout
    with open(out, "wb") as f:
        p = subprocess.run(argv, stdout=f, stderr=subprocess.PIPE, timeout=timeout)
    return p.returncode, p.stderr.decode(errors="replace"), None


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def load_json(path):
    with open(path, "rb") as f:
        return json.load(f)


def engine_line(stderr):
    for line in stderr.splitlines():
        if line.startswith("engine: "):
            return line.strip()
    return stderr.strip().splitlines()[-1] if stderr.strip() else ""


def bucket_refusal(stderr):
    """(bucket, row, message prefix with digits masked) for a `tag` refusal."""
    line = engine_line(stderr)
    for bucket, row, pattern in REFUSALS:
        if re.search(pattern, line):
            return bucket, row, line
    masked = re.sub(r"\d+", "N", re.sub(r"`[^`]*`", "`…`", line))
    return "other", "-", masked[:120]


def npages(qpdf, pdf):
    rc, _, out = run([qpdf, "--show-npages", pdf])
    try:
        return int(out.decode().strip()) if rc in (0, 3) else None
    except ValueError:
        return None


def qpdf_objects(qpdf, pdf):
    rc, _, out = run([qpdf, "--json=2", "--json-key=qpdf", pdf])
    if rc not in (0, 3):
        return None
    try:
        return json.loads(out)["qpdf"][1]
    except (ValueError, KeyError, IndexError):
        return None


def producer_of(objs):
    if not objs:
        return None
    info = objs.get("trailer", {}).get("value", {}).get("/Info")
    if not isinstance(info, str):
        return "(no /Info)"
    value = objs.get("obj:" + info, {}).get("value", {})
    p = value.get("/Producer") if isinstance(value, dict) else None
    if p is None:
        return "(no /Producer)"
    if isinstance(p, str) and p.startswith("u:"):
        return p[2:]
    if isinstance(p, str) and p.startswith("b:"):
        return bytes.fromhex(p[2:]).decode("latin-1")
    return str(p)


# --- the tree the writer wrote ---------------------------------------------------------------------

def tree_counts(objs):
    """Elements by role, sequences per `/Div` from `/K`, the stamp and the parent tree's next key."""
    roles = Counter()
    per_block = []
    k_not_ints = 0
    stamp = False
    next_key = None
    for obj in objs.values():
        v = obj.get("value") if isinstance(obj, dict) else None
        if not isinstance(v, dict):
            continue
        if v.get("/Type") == "/Catalog":
            stamp = "/EthosParserTags" in v
        if v.get("/Type") == "/StructTreeRoot":
            next_key = v.get("/ParentTreeNextKey")
        if v.get("/Type") != "/StructElem":
            continue
        role = v.get("/S")
        roles[role] += 1
        if role == "/Div":
            k = v.get("/K", [])
            if not isinstance(k, list):
                k = [k]
            ints = [x for x in k if isinstance(x, int)]
            if len(ints) != len(k):
                k_not_ints += 1
            per_block.append(len(ints))
    per_block.sort()
    return {
        "elements": sum(roles.values()),
        "roles": dict(roles),
        "blocks": len(per_block),
        "sequences": sum(per_block),
        "seq_per_block_min": per_block[0] if per_block else 0,
        "seq_per_block_median": per_block[len(per_block) // 2] if per_block else 0,
        "seq_per_block_max": per_block[-1] if per_block else 0,
        "blocks_multi": sum(1 for n in per_block if n > 1),
        "k_not_ints": k_not_ints,
        "stamp": stamp,
        "parent_tree_next_key": next_key,
    }


# --- reals outside content streams ------------------------------------------------------------------

def shortest_f32(text):
    """The shortest decimal that round-trips to the nearest f32 of `text` — what a writer printing
    f32 shortest-round-trip emits — or None if the value does not fit an f32."""
    try:
        v32 = struct.unpack("<f", struct.pack("<f", float(text)))[0]
    except (OverflowError, ValueError):
        return None
    for digits in range(1, 10):
        s = "%.*g" % (digits, v32)
        try:
            if struct.unpack("<f", struct.pack("<f", float(s)))[0] == v32:
                return s
        except OverflowError:
            return None
    return repr(v32)


def survives_f32(text):
    s = shortest_f32(text)
    if s is None:
        return False
    try:
        return Decimal(s) == Decimal(text)
    except InvalidOperation:
        return False


def scan_reals(pdf_bytes):
    """Per key: reals seen and reals that do not survive f32, with one example of each loss."""
    body = STREAM_DATA.sub(b"stream\nendstream", pdf_bytes)
    seen = Counter()
    lost = Counter()
    examples = {}
    for m in KEY_ARRAY.finditer(body):
        key = m.group(1).decode()
        for tok in TOKEN.findall(m.group(2)):
            if not REAL.match(tok):
                continue
            text = tok.decode()
            seen[key] += 1
            if not survives_f32(text):
                lost[key] += 1
                examples.setdefault(key, text + " -> " + str(shortest_f32(text)))
    return {
        "seen": dict(seen),
        "lost": dict(lost),
        "examples": examples,
        "objstm": b"/ObjStm" in pdf_bytes,
        "widths_indirect": len(re.findall(rb"/Widths\s+\d+\s+\d+\s+R", body)),
    }


# --- projection equality ------------------------------------------------------------------------------

def canonical(obj):
    return json.dumps(obj, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")


def comparable(kind, art):
    art = json.loads(json.dumps(art))
    art.pop("assurance", None)
    if isinstance(art.get("representation"), dict):
        art["representation"].pop("assurance", None)
    if kind == "ground":
        if isinstance(art.get("source"), dict):
            art["source"].pop("sha256", None)
    else:
        art.pop("source_sha256", None)
        art.pop("representation_sha256", None)
    return art


def first_difference(a, b, path=""):
    """The first differing field in sorted-key order as (path, description), or None."""
    if isinstance(a, dict) and isinstance(b, dict):
        for k in sorted(set(a) | set(b)):
            if k not in a or k not in b:
                return path + "/" + k, "present on one side only"
            d = first_difference(a[k], b[k], path + "/" + k)
            if d:
                return d
        return None
    if isinstance(a, list) and isinstance(b, list):
        for i, (x, y) in enumerate(zip(a, b)):
            d = first_difference(x, y, path + "[%d]" % i)
            if d:
                return d
        if len(a) != len(b):
            return path, "length %d against %d" % (len(a), len(b))
        return None
    if a != b:
        return path, "%s against %s" % (json.dumps(a)[:60], json.dumps(b)[:60])
    return None


def compare_projection(kind, tagged_path, original_path):
    t = comparable(kind, load_json(tagged_path))
    o = comparable(kind, load_json(original_path))
    rec = {"equal": canonical(t) == canonical(o)}
    if rec["equal"]:
        return rec
    d = first_difference(t, o)
    rec["first_difference"] = d[0] + ": " + d[1] if d else "(canonical bytes differ, no field found)"
    if kind == "ground":
        rec["elements"] = [len(t.get("elements", [])), len(o.get("elements", []))]
        rec["text_differs"] = [e.get("text") for e in t.get("elements", [])] != [e.get("text") for e in o.get("elements", [])]
    else:
        rec["text_differs"] = t.get(kind) != o.get(kind)
        st = t.get("anchor_map", {}).get("segments", [])
        so = o.get("anchor_map", {}).get("segments", [])
        rec["segments"] = [len(st), len(so)]
        rec["segments_differing"] = sum(1 for x, y in zip(st, so) if x != y) + abs(len(st) - len(so))
    return rec


# --- the representation ---------------------------------------------------------------------------------

CODES_OF_INTEREST = (
    "structure-tree-engine-written",
    "untagged-structure-tree-absent",
    "mcid-property-list-by-name",
    "inline-images-not-emitted",
)


def limitation_codes(art):
    """The codes of CODES_OF_INTEREST the artifact declares, and how many codes it declares."""
    try:
        codes = sorted(l["code"] for l in art["representation"]["assurance"]["limitations"])
    except (KeyError, TypeError):
        return []
    return [c for c in codes if c in CODES_OF_INTEREST] + ["(%d codes declared)" % len(codes)]


def run_counts(art):
    """Runs, runs bound pdf_tagged, of those with role path Document/Div, carrying `derivation`, and
    the derivation values."""
    rep = art.get("representation", {})
    runs = [n for n in rep.get("nodes", []) if n.get("kind") == "text_run"]
    tagged = [n["structural_locator"]["pdf_tagged"] for n in runs
              if isinstance(n.get("structural_locator"), dict) and "pdf_tagged" in n["structural_locator"]]
    return {
        "runs": len(runs),
        "pdf_tagged": len(tagged),
        "document_div": sum(1 for t in tagged if t.get("role_path") == ["Document", "Div"]),
        "with_derivation": sum(1 for t in tagged if "derivation" in t),
        "derivations": dict(Counter(t.get("derivation", "(absent)") for t in tagged)),
        "artifact": sum(1 for n in runs if isinstance(n.get("structural_locator"), dict) and "pdf_artifact" in n["structural_locator"]),
        "mcid": sum(1 for n in runs if isinstance(n.get("structural_locator"), dict) and "pdf_mcid" in n["structural_locator"]),
        "limitations": limitation_codes(art),
    }


def first_block_run(art):
    """The first non-whitespace run bound pdf_tagged, in node order: its text and page index."""
    rep = art.get("representation", {})
    pages = {p["id"]: p["index"] for p in rep.get("pages", [])}
    for n in rep.get("nodes", []):
        if n.get("kind") != "text_run":
            continue
        loc = n.get("structural_locator")
        if not (isinstance(loc, dict) and "pdf_tagged" in loc):
            continue
        if not n.get("text", "").strip():
            continue
        attrs = n.get("attributes", {}).get("text_run", {})
        return {"node": n["id"], "text": n["text"], "page": pages.get(n.get("parent")),
                "region": attrs.get("region"), "block": attrs.get("block")}
    return None


# --- ethos verify ----------------------------------------------------------------------------------------

def verify_one(ethos, grounding_path, claims_path, run_text, page_index):
    """Cite `run_text` at the first element on page `page_index` whose text contains it; verify."""
    g = load_json(grounding_path)
    page_id = "p%d" % page_index if page_index is not None else None
    element = None
    for e in g.get("elements", []):
        if e.get("page") == page_id and run_text in e.get("text", ""):
            element = e
            break
    if element is None:
        return {"outcome": "no-element-contains-the-run"}
    claims = {
        "document_fingerprint": "sha256:" + sha256_file(grounding_path),
        "claims": [{"kind": "quote", "text": run_text,
                    "citation": {"page": page_id, "element_id": element["id"]}}],
    }
    with open(claims_path, "w", encoding="utf-8") as f:
        json.dump(claims, f, ensure_ascii=False)
    rc, err, out = run([ethos, "verify", grounding_path, "--citations", claims_path,
                        "--grounding", "ethos-grounding-json"])
    rec = {"exit": rc, "element": element["id"], "element_text_len": len(element.get("text", ""))}
    try:
        report = json.loads(out)
    except ValueError:
        rec["outcome"] = "no-report"
        rec["stderr"] = err.strip()[-200:]
        return rec
    pred = report.get("predicate", report)
    check = (pred.get("checks") or [{}])[0]
    rec["all_evidence_grounded"] = pred.get("all_evidence_grounded")
    rec["fingerprint_stale"] = pred.get("fingerprint_stale")
    rec["status"] = check.get("status")
    rec["evidence_tier"] = check.get("evidence_tier")
    rec["match_method"] = check.get("match_method")
    rec["outcome"] = "report"
    return rec


def verify_agrees(a, b):
    """True when both reports agree, None when neither grounding artifact has an element holding
    the run (no measurable ink on either side), False otherwise — one side having an element the
    other lacks is a disagreement between the artifacts."""
    if a.get("outcome") == "no-element-contains-the-run" and b.get("outcome") == "no-element-contains-the-run":
        return None
    if a.get("outcome") != "report" or b.get("outcome") != "report":
        return False
    return all(a.get(k) == b.get(k) for k in ("all_evidence_grounded", "status", "evidence_tier", "match_method"))


# --- one document -----------------------------------------------------------------------------------------

def name_of(corpus, root, pdf):
    rel = os.path.relpath(pdf, root)
    if os.path.basename(rel) in ("document.pdf", "source.pdf"):
        rel = os.path.dirname(rel)
    else:
        rel = rel.rsplit(".", 1)[0]
    return corpus + "/" + rel.replace(os.sep, "/")


def one(job):
    corpus, root, pdf, cfg = job
    name = name_of(corpus, root, pdf)
    d = os.path.join(cfg["out"], name)
    os.makedirs(d, exist_ok=True)
    branch, release, ethos, qpdf = cfg["branch"], cfg["release"], cfg["ethos"], cfg["qpdf"]
    started = time.time()
    rec = {"name": name, "corpus": corpus, "pages": cfg["pages"].get(pdf),
           "input_bytes": os.path.getsize(pdf)}

    with open(pdf, "rb") as f:
        original_bytes = f.read()
    rec["reals"] = scan_reals(original_bytes)
    rec["producer"] = producer_of(qpdf_objects(qpdf, pdf))

    # The original's own extract, for every document.
    o_json = os.path.join(d, "original.extract.json")
    rec["original_extract_exit"], err, _ = run([branch, "extract", pdf], o_json)
    if rec["original_extract_exit"] == 0:
        o_art = load_json(o_json)
        rec["original"] = run_counts(o_art)
    else:
        rec["original"] = {"error": engine_line(err)[:200]}

    # 1. tag
    tagged = os.path.join(d, "tagged.pdf")
    rec["tag_exit"], err, _ = run([branch, "tag", pdf], tagged)
    if rec["tag_exit"] != 0:
        rec["refusal"], rec["refusal_row"], rec["refusal_message"] = bucket_refusal(err)
        rec["output_bytes"] = os.path.getsize(tagged)
        rec["seconds"] = round(time.time() - started, 2)
        return rec
    rec["refusal"] = None
    rec["output_bytes"] = os.path.getsize(tagged)
    rec["bytes_added"] = rec["output_bytes"] - rec["input_bytes"]
    # Byte identity across runs (milestones S2): a second `tag`, compared byte for byte.
    again = os.path.join(d, "tagged-again.pdf")
    again_exit, _, _ = run([branch, "tag", pdf], again)
    rec["identical_across_runs"] = again_exit == 0 and sha256_file(again) == sha256_file(tagged)
    os.remove(again)

    # 2. the tree
    objs = qpdf_objects(qpdf, tagged)
    rec["tree"] = tree_counts(objs) if objs else {"error": "qpdf --json failed on the output"}

    # 3. extract both, project both, compare
    t_json = os.path.join(d, "tagged.extract.json")
    rec["tagged_extract_exit"], err, _ = run([branch, "extract", tagged], t_json)
    if rec["tagged_extract_exit"] != 0:
        rec["tagged"] = {"error": engine_line(err)[:200]}
    else:
        t_art = load_json(t_json)
        rec["tagged"] = run_counts(t_art)
        rec["projections"] = {}
        for kind in ("ground", "markdown", "html"):
            tp = os.path.join(d, "tagged." + kind + ".json")
            op = os.path.join(d, "original." + kind + ".json")
            te, _, _ = run([branch, kind, t_json], tp)
            oe = None
            if rec["original_extract_exit"] == 0:
                oe, _, _ = run([branch, kind, o_json], op)
            p = {"tagged_exit": te, "original_exit": oe}
            if te == 0 and oe == 0:
                p.update(compare_projection(kind, tp, op))
            rec["projections"][kind] = p

        # 4. grounding-check on the tagged grounding artifact, bound to the tagged bytes
        gp = os.path.join(d, "tagged.ground.json")
        if rec["projections"]["ground"]["tagged_exit"] == 0:
            rc, err, out = run([branch, "grounding-check", gp, "--source-artifact", tagged])
            gc = {"exit": rc}
            try:
                v = json.loads(out)
                gc["structure"] = v.get("structure")
                gc["source_binding"] = v.get("source_binding")
                gc["elements"] = v.get("counts", {}).get("elements")
            except ValueError:
                gc["stderr"] = err.strip()[-200:]
            rec["grounding_check"] = gc

            # 5. ethos verify, the same claim against both grounding artifacts
            first = first_block_run(t_art)
            rec["claim"] = first
            if first and rec["projections"]["ground"]["original_exit"] == 0:
                vt = verify_one(ethos, gp, os.path.join(d, "claims.tagged.json"), first["text"], first["page"])
                vo = verify_one(ethos, os.path.join(d, "original.ground.json"),
                                os.path.join(d, "claims.original.json"), first["text"], first["page"])
                rec["verify"] = {"tagged": vt, "original": vo, "agree": verify_agrees(vt, vo)}

    # 6. the consumer that ignores /A: the release binary on the writer's output
    r_json = os.path.join(d, "release.extract.json")
    rc, err, _ = run([release, "extract", tagged], r_json)
    rec["release"] = run_counts(load_json(r_json)) if rc == 0 else {"error": engine_line(err)[:200]}
    rec["release"]["exit"] = rc

    rec["seconds"] = round(time.time() - started, 2)
    return rec


# --- the gate: the wire cost of `derivation`, computed ---------------------------------------------------

def count_bytes(path, needle):
    n = 0
    carry = b""
    with open(path, "rb") as f:
        while True:
            block = f.read(1 << 24)
            if not block:
                break
            buf = carry + block
            n += buf.count(needle)
            carry = buf[-(len(needle) - 1):]
    return n


def gate_cost(cfg, root, pdf):
    name = name_of("gate", root, pdf)
    d = os.path.join(cfg["out"], name)
    os.makedirs(d, exist_ok=True)
    art = os.path.join(d, "branch.extract.json")
    rec = {"name": name, "pdf": pdf, "pages": cfg["pages"].get(pdf), "timeout": cfg["gate_timeout"]}
    started = time.time()
    try:
        rec["extract_exit"], err, _ = run([cfg["branch"], "extract", pdf], art, timeout=cfg["gate_timeout"])
    except subprocess.TimeoutExpired:
        rec["skipped"] = "extract exceeded %d s" % cfg["gate_timeout"]
        rec["seconds"] = round(time.time() - started, 1)
        return rec
    rec["seconds"] = round(time.time() - started, 1)
    if rec["extract_exit"] != 0:
        rec["error"] = engine_line(err)[:200]
        return rec
    rec["artifact_bytes"] = os.path.getsize(art)
    rec["pdf_tagged_locators"] = count_bytes(art, PDF_TAGGED_KEY)
    rec["derivation_bytes_each"] = len(DERIVATION_KEY)
    rec["derivation_bytes"] = rec["pdf_tagged_locators"] * len(DERIVATION_KEY)
    rec["share"] = rec["derivation_bytes"] / rec["artifact_bytes"] if rec["artifact_bytes"] else None
    os.remove(art)
    # The same document under the release binary: the artifact bytes the field is added to.
    rel = os.path.join(d, "release.extract.json")
    try:
        rec["release_extract_exit"], err, _ = run([cfg["release"], "extract", pdf], rel, timeout=cfg["gate_timeout"])
    except subprocess.TimeoutExpired:
        rec["release_skipped"] = "extract exceeded %d s" % cfg["gate_timeout"]
        return rec
    if rec["release_extract_exit"] == 0:
        rec["release_artifact_bytes"] = os.path.getsize(rel)
        rec["release_pdf_tagged_locators"] = count_bytes(rel, PDF_TAGGED_KEY)
    os.remove(rel)
    return rec


# --- the summary --------------------------------------------------------------------------------------------

def fmt(n):
    return "{:,}".format(n) if isinstance(n, int) else str(n)


def pct(n, d):
    return "%.1f%%" % (100.0 * n / d) if d else "-"


def worst(records, key, reverse=True):
    """The record maximising key(rec) (or minimising with reverse=False), ties by name."""
    scored = [(key(r), r["name"]) for r in records if key(r) is not None]
    if not scored:
        return None
    scored.sort(key=lambda t: (-t[0] if reverse else t[0], t[1]))
    return scored[0]


def summarise(records, gate, run_info):
    corpora = sorted({r["corpus"] for r in records})
    tagged = [r for r in records if r.get("refusal") is None]
    refused = [r for r in records if r.get("refusal") is not None]
    s = {"run": run_info, "corpora": corpora, "documents": len(records), "tagged": len(tagged), "refused": len(refused)}

    # refusals by bucket and corpus
    by = defaultdict(Counter)
    rows = {}
    others = Counter()
    for r in refused:
        by[r["refusal"]][r["corpus"]] += 1
        rows[r["refusal"]] = r["refusal_row"]
        if r["refusal"] == "other":
            others[r["refusal_message"]] += 1
    s["refusals"] = {b: {"row": rows[b], "by_corpus": dict(c), "total": sum(c.values())} for b, c in by.items()}
    s["refusals_other"] = dict(others)
    s["tagged_by_corpus"] = dict(Counter(r["corpus"] for r in tagged))
    s["documents_by_corpus"] = dict(Counter(r["corpus"] for r in records))

    # the tree
    trees = [r for r in tagged if "tree" in r and "error" not in r["tree"]]
    blocks = sum(r["tree"]["blocks"] for r in trees)
    seqs = sum(r["tree"]["sequences"] for r in trees)
    multi = sum(r["tree"]["blocks_multi"] for r in trees)
    s["tree"] = {
        "documents": len(trees),
        "elements": sum(r["tree"]["elements"] for r in trees),
        "blocks": blocks,
        "sequences": seqs,
        "blocks_multi": multi,
        "blocks_multi_share": multi / blocks if blocks else None,
        "max_seq_per_block": worst(trees, lambda r: r["tree"]["seq_per_block_max"]),
        "max_multi_share": worst(trees, lambda r: r["tree"]["blocks_multi"] / r["tree"]["blocks"] if r["tree"]["blocks"] else None),
        "roles": dict(sum((Counter(r["tree"]["roles"]) for r in trees), Counter())),
        "stamp_missing": [r["name"] for r in trees if not r["tree"]["stamp"]],
        "k_not_ints": sum(r["tree"]["k_not_ints"] for r in trees),
        "per_doc_median_max": worst(trees, lambda r: r["tree"]["seq_per_block_median"]),
    }

    # limitations of the original, by outcome
    lim = {}
    for code in ("mcid-property-list-by-name", "inline-images-not-emitted"):
        carrying = [r for r in records if code in r.get("original", {}).get("limitations", [])]
        lim[code] = {
            "documents": len(carrying),
            "tagged": sum(1 for r in carrying if r.get("refusal") is None),
            "refused": dict(Counter(r["refusal"] for r in carrying if r.get("refusal") is not None)),
            "names": [r["name"] for r in carrying][:12],
        }
    s["original_limitations"] = lim

    # bytes added
    sized = [r for r in tagged if "bytes_added" in r]
    if sized:
        rel = [(r["bytes_added"] / r["input_bytes"], r["name"]) for r in sized if r["input_bytes"]]
        rel.sort()
        s["bytes_added"] = {
            "documents": len(sized),
            "abs_min": worst(sized, lambda r: r["bytes_added"], reverse=False),
            "abs_max": worst(sized, lambda r: r["bytes_added"]),
            "abs_median": sorted(r["bytes_added"] for r in sized)[len(sized) // 2],
            "rel_min": rel[0], "rel_max": rel[-1], "rel_median": rel[len(rel) // 2],
            "total_in": sum(r["input_bytes"] for r in sized),
            "total_out": sum(r["output_bytes"] for r in sized),
            "shrank": sum(1 for r in sized if r["bytes_added"] < 0),
            "identical_across_runs": sum(1 for r in sized if r.get("identical_across_runs")),
            "differing_across_runs": [r["name"] for r in sized if not r.get("identical_across_runs")],
        }

    # reals
    seen = Counter()
    lost = Counter()
    by_producer = Counter()
    by_producer_seen = Counter()
    examples = {}
    for r in records:
        seen.update(r["reals"]["seen"])
        lost.update(r["reals"]["lost"])
        n = sum(r["reals"]["lost"].values())
        by_producer[r.get("producer") or "(unknown)"] += n
        by_producer_seen[r.get("producer") or "(unknown)"] += sum(r["reals"]["seen"].values())
        for k, v in r["reals"]["examples"].items():
            examples.setdefault(k, (r["name"], v))
    s["reals"] = {
        "documents": len(records),
        "seen": dict(seen), "lost": dict(lost),
        "seen_total": sum(seen.values()), "lost_total": sum(lost.values()),
        "worst_document": worst(records, lambda r: sum(r["reals"]["lost"].values())),
        "producers_lost": by_producer.most_common(5),
        "producers_seen": dict(by_producer_seen),
        "examples": examples,
        "objstm_documents": [r["name"] for r in records if r["reals"]["objstm"]],
        "widths_indirect": sum(r["reals"]["widths_indirect"] for r in records),
        "documents_with_loss": sum(1 for r in records if sum(r["reals"]["lost"].values())),
    }

    # projections
    proj = {}
    for kind in ("ground", "markdown", "html"):
        have = [r for r in tagged if "projections" in r and "equal" in r["projections"].get(kind, {})]
        diff = [r for r in have if not r["projections"][kind]["equal"]]
        p = {"compared": len(have), "equal": len(have) - len(diff), "different": len(diff),
             "text_differs": sum(1 for r in diff if r["projections"][kind].get("text_differs")),
             "not_compared": len(tagged) - len(have),
             "first_differences": dict(Counter(re.sub(r"\[\d+\]", "[i]", r["projections"][kind]["first_difference"].split(":")[0]) for r in diff))}
        if kind == "ground":
            p["worst"] = worst(diff, lambda r: abs(r["projections"][kind]["elements"][0] - r["projections"][kind]["elements"][1]))
            p["elements_total"] = [sum(r["projections"][kind]["elements"][0] for r in diff), sum(r["projections"][kind]["elements"][1] for r in diff)]
        else:
            p["worst"] = worst(diff, lambda r: r["projections"][kind]["segments_differing"])
        if diff:
            ex = min(diff, key=lambda r: r["name"])
            p["example"] = (ex["name"], ex["projections"][kind]["first_difference"])
        proj[kind] = p
    s["projections"] = proj

    # grounding-check
    gc = [r for r in tagged if "grounding_check" in r]
    s["grounding_check"] = {
        "run": len(gc),
        "exit_codes": dict(Counter(r["grounding_check"]["exit"] for r in gc)),
        "structure": dict(Counter(r["grounding_check"].get("structure") for r in gc)),
        "source_binding": dict(Counter(r["grounding_check"].get("source_binding") for r in gc)),
        "not_run": [r["name"] for r in tagged if "grounding_check" not in r],
    }

    # verify
    ver = [r for r in tagged if "verify" in r and r["verify"]["agree"] is not None]
    no_ink = [r["name"] for r in tagged if "verify" in r and r["verify"]["agree"] is None]
    s["verify"] = {
        "run": len(ver),
        "no_element_either_side": len(no_ink),
        "agree": sum(1 for r in ver if r["verify"]["agree"]),
        "disagree": [r["name"] for r in ver if not r["verify"]["agree"]],
        "grounded_both": sum(1 for r in ver if r["verify"]["tagged"].get("status") == "grounded" == r["verify"]["original"].get("status")),
        "outcomes_tagged": dict(Counter(r["verify"]["tagged"].get("status", r["verify"]["tagged"].get("outcome")) for r in ver)),
        "outcomes_original": dict(Counter(r["verify"]["original"].get("status", r["verify"]["original"].get("outcome")) for r in ver)),
        "not_run": [r["name"] for r in tagged if "verify" not in r],
    }
    engine_ink = [r for r in ver if r["corpus"] == "engine"]
    bench_first = sorted((r for r in ver if r["corpus"] == "bench"), key=lambda r: r["name"])[:10]
    s["verify"]["named_subset"] = {
        "engine": [(r["name"], r["verify"]["tagged"].get("status"), r["verify"]["original"].get("status"), r["verify"]["agree"]) for r in engine_ink],
        "bench": [(r["name"], r["verify"]["tagged"].get("status"), r["verify"]["original"].get("status"), r["verify"]["agree"]) for r in bench_first],
    }

    # the release consumer
    rel = [r for r in tagged if "release" in r and r["release"].get("exit") == 0]
    s["release"] = {
        "run": len(rel),
        "runs": sum(r["release"]["runs"] for r in rel),
        "pdf_tagged": sum(r["release"]["pdf_tagged"] for r in rel),
        "document_div": sum(r["release"]["document_div"] for r in rel),
        "with_derivation": sum(r["release"]["with_derivation"] for r in rel),
        "all_bound": sum(1 for r in rel if r["release"]["pdf_tagged"] + r["release"]["artifact"] == r["release"]["runs"]),
        "failed": [r["name"] for r in tagged if "release" in r and r["release"].get("exit") != 0],
        "limitation_codes": dict(sum((Counter(r["release"]["limitations"]) for r in rel), Counter())),
    }
    named = [r for r in rel if r["name"] == "engine/leading-gap-two-blocks"]
    bench_one = sorted((r for r in rel if r["corpus"] == "bench"), key=lambda r: r["name"])[:1]
    s["release"]["named"] = [
        {"name": r["name"], "release": r["release"], "branch": r.get("tagged")} for r in named + bench_one
    ]

    # the branch's own read-back, as a count
    br = [r for r in tagged if "tagged" in r and "error" not in r["tagged"]]
    s["branch_readback"] = {
        "documents": len(br),
        "runs": sum(r["tagged"]["runs"] for r in br),
        "pdf_tagged": sum(r["tagged"]["pdf_tagged"] for r in br),
        "computed": sum(r["tagged"]["derivations"].get("computed", 0) for r in br),
        "document_div": sum(r["tagged"]["document_div"] for r in br),
        "engine_written_declared": sum(1 for r in br if "structure-tree-engine-written" in r["tagged"]["limitations"]),
        "absent_declared": sum(1 for r in br if "untagged-structure-tree-absent" in r["tagged"]["limitations"]),
    }

    s["gate"] = gate
    s["seconds"] = round(sum(r.get("seconds", 0) for r in records), 1)
    return s


def render(s):
    out = []
    w = out.append
    w("Run %s: commit %s, branch binary %s, release binary %s, ethos %s, qpdf %s, python %s." % (
        s["run"]["date"], s["run"]["commit"], s["run"]["branch_version"], s["run"]["release_version"],
        s["run"]["ethos_version"], s["run"]["qpdf_version"], s["run"]["python_version"]))
    w("")
    w("documents: %d; tagged: %d; refused: %d (%.1f s of document work)" % (s["documents"], s["tagged"], s["refused"], s["seconds"]))
    w("")
    w("| outcome | §3.6 row | " + " | ".join(s["corpora"]) + " | all |")
    w("| --- | --- | " + " | ".join("---:" for _ in s["corpora"]) + " | ---: |")
    w("| tagged | - | " + " | ".join(fmt(s["tagged_by_corpus"].get(c, 0)) for c in s["corpora"]) + " | %s |" % fmt(s["tagged"]))
    for b, v in sorted(s["refusals"].items(), key=lambda kv: -kv[1]["total"]):
        w("| refused: %s | %s | " % (b, v["row"]) + " | ".join(fmt(v["by_corpus"].get(c, 0)) for c in s["corpora"]) + " | %s |" % fmt(v["total"]))
    w("| documents | - | " + " | ".join(fmt(s["documents_by_corpus"].get(c, 0)) for c in s["corpora"]) + " | %s |" % fmt(s["documents"]))
    if s["refusals_other"]:
        w("")
        w("`other` refusal messages (digits masked):")
        for m, n in sorted(s["refusals_other"].items()):
            w("- %d: `%s`" % (n, m))
    t = s["tree"]
    w("")
    w("| the tree, over %d tagged documents | value |" % t["documents"])
    w("| --- | ---: |")
    w("| elements written (`/StructElem`) | %s |" % fmt(t["elements"]))
    w("| roles | %s |" % ", ".join("%s %s" % (k, fmt(v)) for k, v in sorted(t["roles"].items())))
    w("| `/Div` blocks | %s |" % fmt(t["blocks"]))
    w("| sequences written (`/K` ids) | %s |" % fmt(t["sequences"]))
    w("| blocks with more than one sequence | %s (%s) |" % (fmt(t["blocks_multi"]), pct(t["blocks_multi"], t["blocks"])))
    w("| most sequences in one block | %s (`%s`) |" % (fmt(t["max_seq_per_block"][0]), t["max_seq_per_block"][1]) if t["max_seq_per_block"] else "| most sequences in one block | - |")
    w("| highest per-document share of multi-sequence blocks | %s (`%s`) |" % (pct(t["max_multi_share"][0], 1), t["max_multi_share"][1]) if t["max_multi_share"] else "| highest share | - |")
    w("| `/K` entries that are not bare ids | %s |" % fmt(t["k_not_ints"]))
    w("| catalog stamp missing | %s |" % (", ".join(t["stamp_missing"]) or "none"))
    w("")
    for code, v in s["original_limitations"].items():
        w("`%s` in the original's artifact: %d documents, %d tagged, refused %s%s" % (
            code, v["documents"], v["tagged"], json.dumps(v["refused"]), (" — " + ", ".join(v["names"])) if v["names"] else ""))
    if "bytes_added" in s:
        b = s["bytes_added"]
        w("")
        w("| bytes added, over %d tagged documents | min | median | max |" % b["documents"])
        w("| --- | ---: | ---: | ---: |")
        w("| absolute | %s (`%s`) | %s | %s (`%s`) |" % (fmt(b["abs_min"][0]), b["abs_min"][1], fmt(b["abs_median"]), fmt(b["abs_max"][0]), b["abs_max"][1]))
        w("| relative to the input | %s (`%s`) | %s | %s (`%s`) |" % (pct(b["rel_min"][0], 1), b["rel_min"][1], pct(b["rel_median"][0], 1), pct(b["rel_max"][0], 1), b["rel_max"][1]))
        w("| total | %s in, %s out, %d documents shrank |  |  |" % (fmt(b["total_in"]), fmt(b["total_out"]), b["shrank"]))
        w("")
        w("A second `tag` of every tagged document: %d of %d byte-identical to the first%s" % (
            b["identical_across_runs"], b["documents"],
            (" — differing: " + ", ".join(b["differing_across_runs"])) if b["differing_across_runs"] else ""))
    r = s["reals"]
    w("")
    w("| reals outside content streams, %d documents | seen | not surviving f32 | first example |" % r["documents"])
    w("| --- | ---: | ---: | --- |")
    for k in ("MediaBox", "CropBox", "Rect", "BBox", "Matrix", "FontMatrix", "Widths"):
        ex = r["examples"].get(k)
        w("| `/%s` | %s | %s | %s |" % (k, fmt(r["seen"].get(k, 0)), fmt(r["lost"].get(k, 0)), ("`%s` (`%s`)" % (ex[1], ex[0])) if ex else ""))
    w("| all | %s | %s | documents with any loss: %d |" % (fmt(r["seen_total"]), fmt(r["lost_total"]), r["documents_with_loss"]))
    w("")
    lost_producers = [(p, n) for p, n in r["producers_lost"] if n]
    w("worst document for reals: %s; producers by reals lost: %s; documents carrying `/ObjStm` (scan partial): %s; `/Widths` given by reference (not scanned): %d; distinct producers: %d" % (
        ("`%s` (%s)" % (r["worst_document"][1], fmt(r["worst_document"][0]))) if r["worst_document"] and r["worst_document"][0] else "none",
        ", ".join("`%s` %s of %s" % (p, fmt(n), fmt(r["producers_seen"].get(p, 0))) for p, n in lost_producers) or "none",
        ", ".join("`%s`" % n for n in r["objstm_documents"]) or "none", r["widths_indirect"], len(r["producers_seen"])))
    w("")
    w("| projection of the tagged representation against the original's | compared | equal | different | projected text differs | worst | first difference (example) |")
    w("| --- | ---: | ---: | ---: | ---: | --- | --- |")
    for kind in ("ground", "markdown", "html"):
        p = s["projections"][kind]
        wd = ("`%s` (%s)" % (p["worst"][1], fmt(p["worst"][0]))) if p.get("worst") else "-"
        ex = ("`%s`: `%s`" % p["example"]) if p.get("example") else "-"
        w("| `%s` | %d | %d | %d | %d | %s | %s |" % (kind, p["compared"], p["equal"], p["different"], p["text_differs"], wd, ex))
    g = s["grounding_check"]
    w("")
    w("grounding-check on %d tagged grounding artifacts: exit codes %s, structure %s, source_binding %s%s" % (
        g["run"], json.dumps(g["exit_codes"]), json.dumps(g["structure"]), json.dumps(g["source_binding"]),
        (" — not run on " + ", ".join(g["not_run"])) if g["not_run"] else ""))
    v = s["verify"]
    w("")
    w("ethos verify, one claim per document against both grounding artifacts: run on %d, reports agree on %d, grounded on both sides on %d, tagged outcomes %s, original outcomes %s%s%s" % (
        v["run"], v["agree"], v["grounded_both"], json.dumps(v["outcomes_tagged"]), json.dumps(v["outcomes_original"]),
        (" — DISAGREE: " + ", ".join(v["disagree"])) if v["disagree"] else "",
        (" — not run on " + ", ".join(v["not_run"])) if v["not_run"] else ""))
    w("")
    w("| named subset | tagged | original | agree |")
    w("| --- | --- | --- | --- |")
    for group in ("engine", "bench"):
        for name, st, so, ag in v["named_subset"][group]:
            w("| `%s` | %s | %s | %s |" % (name, st, so, "yes" if ag else "NO"))
    rl = s["release"]
    w("")
    w("0.58.0 release `extract` on the writer's output, %d documents: runs %s, bound `pdf_tagged` %s, role path Document/Div %s, locators carrying `derivation` %s, documents with every non-artifact run bound %d, limitation codes %s%s" % (
        rl["run"], fmt(rl["runs"]), fmt(rl["pdf_tagged"]), fmt(rl["document_div"]), fmt(rl["with_derivation"]), rl["all_bound"],
        json.dumps(rl["limitation_codes"]), (" — failed on " + ", ".join(rl["failed"])) if rl["failed"] else ""))
    for n in rl["named"]:
        w("- `%s`: release runs %d, pdf_tagged %d, Document/Div %d, with derivation %d, derivations %s; branch runs %d, pdf_tagged %d, derivations %s" % (
            n["name"], n["release"]["runs"], n["release"]["pdf_tagged"], n["release"]["document_div"], n["release"]["with_derivation"], json.dumps(n["release"]["derivations"]),
            n["branch"]["runs"] if n["branch"] else -1, n["branch"]["pdf_tagged"] if n["branch"] else -1, json.dumps(n["branch"]["derivations"]) if n["branch"] else "-"))
    br = s["branch_readback"]
    w("")
    w("branch `extract` on the writer's output, %d documents: runs %s, bound `pdf_tagged` %s, derivation computed %s, Document/Div %s, `structure-tree-engine-written` declared on %d, `untagged-structure-tree-absent` declared on %d" % (
        br["documents"], fmt(br["runs"]), fmt(br["pdf_tagged"]), fmt(br["computed"]), fmt(br["document_div"]), br["engine_written_declared"], br["absent_declared"]))
    if s["gate"]:
        w("")
        w("| gate document | pages | artifact bytes | `pdf_tagged` locators | `derivation` bytes (computed) | share | extract s | release artifact bytes | branch minus release |")
        w("| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |")
        for gr in s["gate"]:
            if "skipped" in gr:
                w("| `%s` | %s | skipped: %s |  |  |  | %s |" % (gr["name"], fmt(gr["pages"]), gr["skipped"], gr["seconds"]))
            elif "error" in gr:
                w("| `%s` | %s | error: %s |  |  |  | %s |" % (gr["name"], fmt(gr["pages"]), gr["error"], gr["seconds"]))
            else:
                rb = gr.get("release_artifact_bytes")
                w("| `%s` | %s | %s | %s | %s | %s | %s | %s | %s |" % (gr["name"], fmt(gr["pages"]), fmt(gr["artifact_bytes"]), fmt(gr["pdf_tagged_locators"]), fmt(gr["derivation_bytes"]), pct(gr["share"], 1), gr["seconds"],
                    fmt(rb) if rb is not None else gr.get("release_skipped", "-"), fmt(gr["artifact_bytes"] - rb) if rb is not None else "-"))
    return "\n".join(out)


# --- main ----------------------------------------------------------------------------------------------------

def pdfs_under(root):
    found = []
    for dirpath, _, files in os.walk(root):
        for f in files:
            if f.lower().endswith(".pdf"):
                found.append(os.path.join(dirpath, f))
    return sorted(found)


def version_of(argv):
    try:
        return subprocess.run(argv, capture_output=True, text=True).stdout.strip().splitlines()[0]
    except (OSError, IndexError):
        return "?"


def main(argv):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--branch", required=True)
    ap.add_argument("--release", required=True)
    ap.add_argument("--ethos", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--results", required=True)
    ap.add_argument("--qpdf", default=os.environ.get("QPDF", "qpdf"))
    ap.add_argument("--jobs", type=int, default=4)
    ap.add_argument("--gate-timeout", type=int, default=300)
    ap.add_argument("--commit", default=None)
    ap.add_argument("corpora", nargs="+", help="LABEL=DIR")
    a = ap.parse_args(argv)

    commit = a.commit
    if commit is None:
        commit = version_of(["git", "-C", os.path.dirname(os.path.abspath(__file__)), "rev-parse", "--short", "HEAD"])
    run_info = {
        "date": time.strftime("%Y-%m-%d"),
        "commit": commit,
        "branch_binary": os.path.abspath(a.branch), "branch_version": version_of([a.branch, "--version"]),
        "release_binary": os.path.abspath(a.release), "release_version": version_of([a.release, "--version"]),
        "ethos_binary": os.path.abspath(a.ethos), "ethos_version": version_of([a.ethos, "--version"]),
        "qpdf_version": version_of([a.qpdf, "--version"]),
        "python_version": sys.version.split()[0],
        "jobs": a.jobs,
    }
    os.makedirs(a.out, exist_ok=True)
    os.makedirs(a.results, exist_ok=True)

    corpora = []
    gate_root = None
    for spec in a.corpora:
        label, root = spec.split("=", 1)
        if label == "gate":
            gate_root = root
        else:
            corpora.append((label, root))
    cfg = {"branch": a.branch, "release": a.release, "ethos": a.ethos, "qpdf": a.qpdf, "out": a.out,
           "gate_timeout": a.gate_timeout, "pages": {}}
    jobs = []
    for label, root in corpora:
        for pdf in pdfs_under(root):
            cfg["pages"][pdf] = npages(a.qpdf, pdf)
            jobs.append((label, root, pdf, cfg))
    run_info["corpora"] = {label: {"root": os.path.abspath(root), "documents": sum(1 for j in jobs if j[0] == label)} for label, root in corpora}

    small = [j for j in jobs if (cfg["pages"].get(j[2]) or 0) <= SEQUENTIAL_ABOVE_PAGES]
    big = [j for j in jobs if (cfg["pages"].get(j[2]) or 0) > SEQUENTIAL_ABOVE_PAGES]
    records = []
    log_path = os.path.join(a.results, "roundtrip.jsonl")
    with open(log_path, "w", encoding="utf-8") as log:
        log.write(json.dumps({"run": run_info}, ensure_ascii=False) + "\n")
        with ProcessPoolExecutor(a.jobs) as pool:
            for rec in pool.map(one, small):
                records.append(rec)
                log.write(json.dumps(rec, ensure_ascii=False) + "\n")
                log.flush()
                print("%-8s %-60s %s" % (rec["corpus"], rec["name"][:60], rec.get("refusal") or "tagged"), file=sys.stderr, flush=True)
        for job in big:
            rec = one(job)
            records.append(rec)
            log.write(json.dumps(rec, ensure_ascii=False) + "\n")
            log.flush()
            print("%-8s %-60s %s" % (rec["corpus"], rec["name"][:60], rec.get("refusal") or "tagged"), file=sys.stderr, flush=True)
        gate = []
        if gate_root:
            for pdf in pdfs_under(gate_root):
                cfg["pages"][pdf] = npages(a.qpdf, pdf)
                gr = gate_cost(cfg, gate_root, pdf)
                gate.append(gr)
                log.write(json.dumps({"gate": gr}, ensure_ascii=False) + "\n")
                log.flush()
                print("gate     %-60s %s" % (gr["name"], gr.get("skipped") or gr.get("error") or ("%s bytes" % fmt(gr.get("artifact_bytes")))), file=sys.stderr, flush=True)

    records.sort(key=lambda r: r["name"])
    summary = summarise(records, gate, run_info)
    with open(os.path.join(a.results, "roundtrip-summary.json"), "w", encoding="utf-8") as f:
        json.dump(summary, f, indent=1, sort_keys=True, ensure_ascii=False)
        f.write("\n")
    print(render(summary))


if __name__ == "__main__":
    main(sys.argv[1:])
