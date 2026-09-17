#!/usr/bin/env python3
r"""Would a `/Div` written per block be read as a paragraph the author did not draw?

# What this measures, and for whom

[`23-AUTO-TAGGING-SCOPE.md`](../../23-AUTO-TAGGING-SCOPE.md) §7.2: decision #23 is re-refused by
*a measured case where a consumer treats an engine-written `/P` as author structure and produces a
citation the document does not support*. The writer emits `/Div`, one per `(page, region, block)`
of `gutter-columns-v3`, and never consults an mcid — so the rate at which a `/Div` read as a
paragraph would cite across a boundary the author drew is fully determined by the untouched
original. This instrument measures that rate on the document as shipped. No writer is involved.

Two shares, each a band across pages with the worst page named (decision #18):

  (i)  blocks whose `/P`-bound runs fall under two or more distinct author `/P` elements, as a
       share of the blocks holding at least one such run — the rate at which a consumer reading
       the written `/Div` as a paragraph cites across a boundary the author drew;
  (ii) author `/P` elements whose runs fall in two or more blocks, as a share of the `/P`
       elements with at least one bound run — the cut splitting a paragraph.

Only `nist-sp-800-207` carries `/P` elements shown to be real paragraphs
([`19-BLOCK-SUBDIVISION-SCOPE.md`](../../19-BLOCK-SUBDIVISION-SCOPE.md) §9.1: 3.63 lines per
`/P`; the other gate producers write one `/P` per line). On the other seven documents the same
two shares describe the producer, not paragraph recall, and the report prints lines per `/P`
beside them so that is visible rather than asserted.

# Method

1. `ethos-parser extract` on the PDF as shipped. Every text run carries its page (through the
   page record its `parent` names), `attributes.text_run.region` and `.block` — either may be
   absent: no region when the vertical cut did not divide the page, no block when the leading-gap
   rule declined the band — and a `structural_locator`: `pdf_tagged` with the `mcid` and the
   author's `role_path`, `pdf_artifact`, or `pdf_mcid` (an id the tree does not cite).
2. `qpdf --json` on the same PDF. The structure tree is walked from `/StructTreeRoot` through
   `/K`: references, arrays, `/MCR` dictionaries, bare integers placed by the nearest `/Pg` (it
   is inheritable). `/RoleMap` is applied to every `/S`. Page objects map to page numbers through
   qpdf's `pages` array, which is the `/Pages` tree in order.
3. Join on `(page, mcid)`. A run belongs to the innermost `/P` above it: walking up from the
   citing element past inline-level elements (PDF 32000-1 §14.8.4.4 Table 338 — a `/Link` or a
   `/Reference` inside a paragraph is part of it), the first block-level element reached is the
   owner, and the run is `/P`-bound when that owner is `/P`. Every other run is excluded and
   counted by reason. Whitespace-only runs are excluded first, as every instrument beside this
   one excludes them, and counted.
4. Aggregate per `(page, region, block)` and per `/P` element, then per page.

# Where it differs from the block-subdivision instruments

`probe3b.py` builds the same join to label consecutive LINE PAIRS, and drops a line whose mcid is
cited by an inline child of a `/P` rather than by the `/P` itself — conservative for a boundary
label. Here the unit is the block and the question is which paragraph a consumer would cite, so
a run cited through `/Link`, `/Reference` or `/Span` inside a `/P` belongs to that `/P`; the
direct-citation rule is printed as a sensitivity line. `SAME_LINE_TOL` (150 centipoints,
`blocks.rs`'s `LINE_TOLERANCE`) and the inline-role set are `labelled.py`'s, except that `Lbl` is
block-level here, as §14.8.4.3.3 Table 336 has it — the README beside `labelled.py` records that
gap. `structelem.py` reads a QDF dump with regular expressions, takes `/S /P` before the
`/RoleMap`, and only an element's own `/Pg`; this reads qpdf's JSON, applies the `/RoleMap` and
inherits `/Pg`. Elements that are `/P` only through the `/RoleMap` are counted separately.

**`structelem.py` cannot see an mcid inside a `/K` array.** Its pattern for array members,
`(?<![\d ])\b(\d+)\b(?!\s+0 R)`, refuses a digit preceded by a space, and `qpdf --qdf` prints
every array member on its own indented line. So `p_elements` keeps only the `/P` elements whose
`/K` is one bare integer — on `nist-sp-800-207`, 314 of the 407 raw `/P` — and a paragraph whose
`/K` is an array (one with a link in it, or one set in several sequences) is dropped whole, not
just its link line as `probe3b.py`'s docstring says. The published 135 boundaries and 719
mid-paragraph pairs are that subset. This instrument's own rows (below) carry every raw `/P`.

# The recall check, which falls out for free

`blocks.rs` publishes 63.7% recall at 100% precision over 135 real boundaries and 719
mid-paragraph pairs, from `probe3b.py` SIMULATING the 1.6x rule on the artifact's baselines. Two
checks are printed. On this instrument's rows — `probe3b.py`'s filters (direct raw `/P`
citations only, digit-only lines dropped, bands of fewer than six lines dropped, the §5
plausibility guard) over every raw `/P` — the SHIPPED cut is scored: `block` changing between
consecutive lines is a fire. With `--probe3b`, `probe3b.py` itself is imported from
`../block-subdivision` and the shipped cut is scored on its exact rows, which separates what the
population changes from what the rule changes.

    paragraphs.py [--probe3b] <extract.json> <source.pdf> [<extract.json> <source.pdf> ...]

`qpdf --json` is run by this script (`QPDF` in the environment names the executable); a second
path ending in `.json` is read as its saved output instead, and `--probe3b` then has no PDF to
run `qpdf --qdf` on and is skipped. The extract is streamed, so the 997 MB artifact of
`nist-sp-800-53Ar5` fits; `--probe3b` loads it whole and is meant for the labelled document.
"""

import io
import json
import os
import re
import subprocess
import sys
from collections import Counter, defaultdict
from pathlib import Path

SAME_LINE_TOL = 150   # centipoints; blocks.rs LINE_TOLERANCE, labelled.py SAME_LINE_TOL
BIN = 10              # probe3b.py: gaps binned to a tenth of a point before the mode is taken
MIN_MODAL_GAP = 600   # probe3b.py: the §5 plausibility guard, lower bound on a modal leading
MIN_SHARE = 0.25      # probe3b.py: the modal bin must hold a quarter of a band's gaps
MIN_ROWS = 6          # probe3b.py: bands with fewer lines are dropped

# PDF 32000-1 §14.8.4.4 Table 338, the inline-level structure elements, plus Em and Strong from
# ISO 32000-2. labelled.py's set without Lbl: Lbl is a list element (Table 336), block-level.
INLINE_ROLES = {
    "Span", "Quote", "Note", "Reference", "BibEntry", "Code", "Link", "Annot",
    "Ruby", "RB", "RT", "RP", "Warichu", "WT", "WP", "Em", "Strong",
}

REF = re.compile(r"^\d+ \d+ R$")
CHUNK = 8 << 20
WORST = 5             # pages named per share


def block_role(path):
    """labelled.py: the nearest block-level role on a role path, or None."""
    for r in reversed(path):
        if r not in INLINE_ROLES:
            return r
    return None


def fmt(n):
    return "{:,}".format(n)


def pct(n, d):
    return "%.1f%%" % (100.0 * n / d) if d else "-"


# --- the extract, streamed (rotcheck.py's reader, plus skip()) -----------------------------------


class Stream:
    def __init__(self, path):
        self.f = io.open(path, "r", encoding="utf-8")
        self.buf = ""
        self.pos = 0
        self.eof = False
        self.dec = json.JSONDecoder()

    def fill(self):
        if self.eof:
            return False
        chunk = self.f.read(CHUNK)
        if not chunk:
            self.eof = True
            return False
        self.buf = self.buf[self.pos:] + chunk
        self.pos = 0
        return True

    def peek(self):
        while self.pos >= len(self.buf):
            if not self.fill():
                return ""
        return self.buf[self.pos]

    def expect(self, ch):
        c = self.peek()
        if c != ch:
            raise ValueError("expected %r at %d, got %r" % (ch, self.pos, c))
        self.pos += 1

    def value(self):
        while True:
            try:
                v, end = self.dec.raw_decode(self.buf, self.pos)
                if end >= len(self.buf) and not self.eof:
                    raise json.JSONDecodeError("edge", self.buf, end)
                self.pos = end
                return v
            except json.JSONDecodeError:
                if not self.fill():
                    v, end = self.dec.raw_decode(self.buf, self.pos)
                    self.pos = end
                    return v

    def key(self):
        k = self.value()
        self.expect(":")
        return k

    def obj_keys(self):
        self.expect("{")
        first = True
        while True:
            if self.peek() == "}":
                self.pos += 1
                return
            if not first:
                self.expect(",")
            first = False
            yield self.key()

    def array_items(self):
        self.expect("[")
        first = True
        while True:
            if self.peek() == "]":
                self.pos += 1
                return
            if not first:
                self.expect(",")
            first = False
            yield self.value()

    def skip(self):
        """Consume a value item by item, so a huge array is never decoded whole."""
        c = self.peek()
        if c == "[":
            for _ in self.array_items():
                pass
        elif c == "{":
            for _ in self.obj_keys():
                self.skip()
        else:
            self.value()


class Run:
    __slots__ = ("parent", "native_page", "y", "region", "block", "kind", "mcid",
                 "engine_role", "text")

    def __init__(self, n):
        loc = (n.get("native_locator") or {}).get("pdf") or {}
        sl = n.get("structural_locator") or {}
        attrs = (n.get("attributes") or {}).get("text_run") or {}
        self.parent = n.get("parent")
        self.native_page = loc.get("page")
        self.y = loc.get("origin_y")
        self.region = attrs.get("region")
        self.block = attrs.get("block")
        self.kind, self.mcid, self.engine_role = None, None, None
        if "pdf_tagged" in sl:
            t = sl["pdf_tagged"]
            self.kind, self.mcid = "pdf_tagged", t.get("mcid")
            self.engine_role = block_role(t.get("standard_role_path") or t.get("role_path") or [])
        elif "pdf_artifact" in sl:
            self.kind = "pdf_artifact"
        elif "pdf_mcid" in sl:
            self.kind, self.mcid = "pdf_mcid", sl["pdf_mcid"]
        self.text = n.get("text", "")


def read_extract(path):
    """`page id -> page number` and one compact record per text run, in node order."""
    s = Stream(path)
    pages, runs = {}, []
    for k in s.obj_keys():
        if k != "representation":
            s.skip()
            continue
        for rk in s.obj_keys():
            if rk == "nodes":
                for n in s.array_items():
                    if n.get("kind") == "text_run":
                        runs.append(Run(n))
            elif rk == "pages":
                for p in s.value():
                    pages[p["id"]] = p["index"]
            else:
                s.skip()
    return pages, runs


# --- the structure tree, from qpdf --json -------------------------------------------------------


class Elem:
    __slots__ = ("raw", "role", "roles")

    def __init__(self, raw, role, roles):
        self.raw, self.role, self.roles = raw, role, roles


class Binding:
    __slots__ = ("owner", "owner_role", "citing", "direct_raw_p")

    def __init__(self, owner, owner_role, citing, direct_raw_p):
        self.owner, self.owner_role, self.citing, self.direct_raw_p = (
            owner, owner_role, citing, direct_raw_p)


def qpdf_json(path):
    if str(path).endswith(".json"):
        return json.load(open(path))
    exe = os.environ.get("QPDF", "qpdf")
    out = subprocess.run([exe, "--json", str(path)], capture_output=True, check=True).stdout
    return json.loads(out)


class Tree:
    def __init__(self, qj):
        self.objs = qj["qpdf"][1]
        self.page_no = {p["object"]: p["pageposfrom1"] for p in qj["pages"]}
        self.rolemap = {}
        self.elements = {}     # element key -> Elem
        self.bindings = {}     # (page, mcid) -> Binding
        self.counts = Counter()
        self.seen = set()
        self.inline_n = 0
        root = self.deref(self.objs["trailer"]["value"]["/Root"])
        st = self.deref(root.get("/StructTreeRoot")) if root else None
        if not isinstance(st, dict):
            self.counts["no /StructTreeRoot"] += 1
            return
        rm = self.deref(st.get("/RoleMap"))
        if isinstance(rm, dict):
            self.rolemap = {k: v for k, v in rm.items() if isinstance(v, str)}
        self.walk_kids(st.get("/K"), None, [])

    def deref(self, v):
        if isinstance(v, str) and REF.match(v):
            o = self.objs.get("obj:" + v)
            return o.get("value") if isinstance(o, dict) else None
        return v

    def mapped(self, s):
        """The standard type a `/S` name maps to through `/RoleMap`, cycle-safe, without `/`."""
        seen = set()
        while s in self.rolemap and s not in seen:
            seen.add(s)
            s = self.rolemap[s]
        return s[1:] if s.startswith("/") else s

    def walk_kids(self, k, pg, chain):
        if k is None:
            return
        for it in (k if isinstance(k, list) else [k]):
            self.walk_kid(it, pg, chain)

    def walk_kid(self, it, pg, chain):
        if isinstance(it, bool):
            self.counts["unrecognised /K item"] += 1
            return
        if isinstance(it, int):
            self.bind(pg, it, chain)
            return
        ref = it if isinstance(it, str) and REF.match(it) else None
        v = self.deref(it) if ref else it
        if isinstance(v, list):
            self.walk_kids(v, pg, chain)
            return
        if not isinstance(v, dict):
            self.counts["unrecognised /K item"] += 1
            return
        t = v.get("/Type")
        if t == "/MCR":
            if "/Stm" in v:
                self.counts["/MCR into a content stream (/Stm)"] += 1
            self.bind(v.get("/Pg", pg), v.get("/MCID"), chain)
            return
        if t == "/OBJR":
            self.counts["/OBJR skipped"] += 1
            return
        if "/S" in v:
            if ref is not None:
                if ref in self.seen:
                    self.counts["element reached twice"] += 1
                    return
                self.seen.add(ref)
                key = ref
            else:
                self.inline_n += 1
                key = "inline#%d" % self.inline_n
            raw = v["/S"]
            role = self.mapped(raw) if isinstance(raw, str) else None
            roles = [r for _, r in chain] + [role]
            self.elements[key] = Elem(raw[1:] if isinstance(raw, str) else None, role, roles)
            if role == "P" and "P" in roles[:-1]:
                self.counts["/P nested in /P"] += 1
            self.walk_kids(v.get("/K"), v.get("/Pg", pg), chain + [(key, role)])
            return
        self.counts["unrecognised /K dictionary"] += 1

    def bind(self, pg, mcid, chain):
        if isinstance(mcid, bool) or not isinstance(mcid, int):
            self.counts["citation without an integer /MCID"] += 1
            return
        page = self.page_no.get(pg)
        if page is None:
            self.counts["citation with no placeable /Pg"] += 1
            return
        owner, owner_role = None, None
        for key, role in reversed(chain):
            if role not in INLINE_ROLES:
                owner, owner_role = key, role
                break
        citing = chain[-1][0]
        if (page, mcid) in self.bindings:
            self.counts["(page, mcid) cited twice"] += 1
            return
        self.bindings[(page, mcid)] = Binding(
            owner, owner_role, citing, self.elements[citing].raw == "P")

    def p_summary(self):
        ps = [e for e in self.elements.values() if e.role == "P"]
        via_map = Counter(e.raw for e in ps if e.raw != "P")
        return len(self.elements), len(ps), via_map


# --- the join ----------------------------------------------------------------------------------


class Joined:
    """Every number the report prints, for one document."""

    def __init__(self, name, pages, runs, tree):
        self.name = name
        self.n_pages = len(pages)
        self.n_runs = len(runs)
        self.tree = tree
        self.reasons = Counter()
        self.page_mismatch = 0
        self.agree = Counter()
        self.bound = []           # (page, region, block, owner key, y, direct)
        self.rows_in = []         # runs the recall check builds its rows from
        self.rows_out = []        # whitespace-only and artifact runs, which the engine still sees
        for r in runs:
            page = pages.get(r.parent)
            if page is None:
                self.reasons["no page record"] += 1
                continue
            if page != r.native_page:
                self.page_mismatch += 1
            blank = not r.text.strip()
            if not blank and r.kind != "pdf_artifact":
                self.rows_in.append(r)
            else:
                self.rows_out.append(r)
            if blank:
                self.reasons["whitespace-only run"] += 1
                continue
            if r.kind == "pdf_artifact":
                self.reasons["artifact (`pdf_artifact`)"] += 1
                continue
            if r.kind == "pdf_mcid":
                self.reasons["id the tree does not cite (`pdf_mcid`)"] += 1
                continue
            if r.kind is None:
                self.reasons["no marked content"] += 1
                continue
            b = tree.bindings.get((page, r.mcid))
            if b is None:
                self.reasons["bound by the engine, not found by this walk"] += 1
                continue
            self.agree["agree" if r.engine_role == b.owner_role else
                       "disagree %s/%s" % (r.engine_role, b.owner_role)] += 1
            if b.owner_role != "P":
                self.reasons["under /" + str(b.owner_role)] += 1
                continue
            self.bound.append((page, r.region, r.block, b.owner, r.y, b.direct_raw_p))
        self.aggregate()

    def in_table(self, e):
        return "Table" in self.tree.elements[e].roles

    def aggregate(self):
        by_block = defaultdict(set)
        by_block_direct = defaultdict(set)
        by_elem = defaultdict(set)
        by_elem_direct = defaultdict(set)
        by_elem_ys = defaultdict(lambda: defaultdict(list))
        for page, region, block, owner, y, direct in self.bound:
            key = (page, region, block)
            by_block[key].add(owner)
            by_elem[owner].add(key)
            by_elem_ys[owner][page].append(y)
            if direct:
                by_block_direct[key].add(owner)
                by_elem_direct[owner].add(key)
        self.by_block, self.by_elem = by_block, by_elem
        self.by_block_direct, self.by_elem_direct = by_block_direct, by_elem_direct
        self.via_inline = sum(1 for b in self.bound if not b[5])
        via_map = [e for e in by_elem if self.tree.elements[e].raw != "P"]
        self.via_rolemap = (len(via_map), sum(1 for b in self.bound if b[3] in set(via_map)))
        self.mixed = {k: v for k, v in by_block.items() if len(v) >= 2}
        self.split = {k: v for k, v in by_elem.items() if len(v) >= 2}
        # (i) split by whether the block holds only /Table-cell paragraphs
        self.table_blocks = {k for k, v in by_block.items() if all(self.in_table(e) for e in v)}
        self.mixed_table = sum(1 for k in self.mixed if k in self.table_blocks)
        self.crossed = sum(len(v) - 1 for v in self.mixed.values())
        self.crossed_prose = sum(len(v) - 1 for k, v in self.mixed.items()
                                 if k not in self.table_blocks)
        self.split_table = sum(1 for e in self.split if self.in_table(e))
        # (ii) by cause
        self.split_cause = Counter()
        for e, keys in self.split.items():
            pages = {k[0] for k in keys}
            bands = {k[:2] for k in keys}
            if len(pages) >= 2:
                self.split_cause["across pages"] += 1
            elif len(bands) >= 2:
                self.split_cause["one page, two or more regions"] += 1
            else:
                self.split_cause["one band, two or more blocks"] += 1
        # per page
        self.page_blocks = defaultdict(lambda: [0, 0])     # page -> [base, mixed]
        self.page_worst_block = {}
        for (page, region, block), v in by_block.items():
            self.page_blocks[page][0] += 1
            if len(v) >= 2:
                self.page_blocks[page][1] += 1
            w = self.page_worst_block.get(page)
            if w is None or len(v) > w[1]:
                self.page_worst_block[page] = ((region, block), len(v))
        self.page_elems = defaultdict(lambda: [0, 0])      # page -> [elements touching, split]
        for e, keys in by_elem.items():
            for page in {k[0] for k in keys}:
                self.page_elems[page][0] += 1
                if len(keys) >= 2:
                    self.page_elems[page][1] += 1
        self.none_blocks = sum(1 for k in by_block if k[2] is None)
        self.none_region = sum(1 for k in by_block if k[1] is None)
        self.mixed_none = sum(1 for k in self.mixed if k[2] is None)
        # lines per /P: distinct baselines among an element's runs, page by page
        lines = []
        for e, by_page in by_elem_ys.items():
            n = 0
            for ys in by_page.values():
                last = None
                for y in sorted(ys):
                    if last is None or y - last > SAME_LINE_TOL:
                        n += 1
                        last = y
            lines.append(n)
        self.lines_per_p = lines
        self.check = self.recall_check()

    def recall_check(self):
        """probe3b.py's rows and filters over every raw /P, scored against the shipped cut."""
        acc = defaultdict(lambda: defaultdict(list))
        for r in self.rows_in:
            acc[(r.native_page, r.region)][r.y].append(r)
        c = Counter()
        for key, by_y in acc.items():
            merged = {}
            for y in sorted(by_y):
                hit = next((m for m in merged if abs(m - y) <= SAME_LINE_TOL), None)
                merged.setdefault(hit if hit is not None else y, []).extend(by_y[y])
            rows = []
            for y, parts in sorted(merged.items()):
                if "".join(p.text for p in parts).strip().isdigit():
                    c["digit-only lines dropped"] += 1
                    continue
                owns = []
                for p in parts:
                    b = self.tree.bindings.get((key[0], p.mcid)) if p.kind == "pdf_tagged" else None
                    if b is not None and b.direct_raw_p:
                        owns.append(b.citing)
                owner = Counter(owns).most_common(1)[0][0] if owns else None
                block = Counter(p.block for p in parts).most_common(1)[0][0]
                rows.append((y, owner, block))
            if len(rows) < MIN_ROWS:
                continue
            gaps = [rows[i + 1][0] - rows[i][0] for i in range(len(rows) - 1)]
            binned = Counter(round(g / BIN) * BIN for g in gaps)
            leading = binned.most_common(1)[0][0] or None
            if leading is None:
                continue
            if leading < MIN_MODAL_GAP or binned[leading] / len(gaps) < MIN_SHARE:
                c["bands failing the plausibility guard"] += 1
                continue
            score_pairs(c, [(o, b) for _, o, b in rows])
        return c

    def probe3b_check(self, extract_path, pdf_path):
        """The shipped cut scored on probe3b.py's own rows, imported from ../block-subdivision."""
        sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "block-subdivision"))
        import probe3b  # noqa: E402
        from labelled import leading_of  # noqa: E402
        art = json.load(open(extract_path))
        owners = probe3b.owner_by_mcid(pdf_path)
        idx = defaultdict(list)
        for r in self.rows_in:
            idx[(r.native_page, r.region)].append((r.y, r.block))
        out = defaultdict(list)
        for r in self.rows_out:
            out[(r.native_page, r.region)].append(r.y)
        c = Counter()
        c["/P elements structelem.py binds"] = len(set(owners.values()))
        for key, rows in probe3b.lines_with_owner(art, owners).items():
            gaps = [rows[i + 1][0] - rows[i][0] for i in range(len(rows) - 1)]
            if not gaps:
                continue
            leading = leading_of(gaps)
            if leading is None:
                continue
            binned = Counter(round(g / probe3b.BIN) * probe3b.BIN for g in gaps)
            if leading < probe3b.MIN_MODAL_GAP or binned[leading] / len(gaps) < probe3b.MIN_SHARE:
                continue
            pairs = []
            for y, owner in rows:
                bs = [b for yy, b in idx[key] if abs(yy - y) <= SAME_LINE_TOL]
                pairs.append((owner, Counter(bs).most_common(1)[0][0] if bs else None))
            score_pairs(c, pairs)
            # Where the simulated rule fires on a boundary and the shipped cut does not: what did
            # the engine see between the two lines that these rows do not carry?
            for i, gap in enumerate(gaps):
                (y0, a), (y1, b) = rows[i], rows[i + 1]
                ba, bb = pairs[i][1], pairs[i + 1][1]
                if a is None or b is None or a == b or gap < 1.6 * leading or ba != bb:
                    continue
                if ba is None:
                    c["missed by the shipped cut: band declined"] += 1
                elif any(y0 + SAME_LINE_TOL < y < y1 - SAME_LINE_TOL for y in out[key]):
                    c["missed by the shipped cut: an excluded run between the lines"] += 1
                else:
                    c["missed by the shipped cut: other"] += 1
            for i, gap in enumerate(gaps):
                (_, a), (_, b) = rows[i], rows[i + 1]
                if a is None or b is None or a == b:
                    continue
                if gap < 1.6 * leading and pairs[i][1] != pairs[i + 1][1]:
                    c["found by the shipped cut, not the simulation"] += 1
        return c

    def share_i(self):
        return len(self.mixed), len(self.by_block)

    def share_ii(self):
        return len(self.split), len(self.by_elem)

    def share_i_direct(self):
        return (sum(1 for v in self.by_block_direct.values() if len(v) >= 2),
                len(self.by_block_direct))

    def share_ii_direct(self):
        return (sum(1 for v in self.by_elem_direct.values() if len(v) >= 2),
                len(self.by_elem_direct))


def score_pairs(c, rows):
    """Consecutive (owner, block) rows: a boundary where the owner changes, a fire where the
    block does; both owners must be known, as in probe3b.py."""
    for (a, ba), (b, bb) in zip(rows, rows[1:]):
        if a is None or b is None:
            continue
        fired = ba != bb
        if a != b:
            c["boundaries"] += 1
            c["boundaries found"] += fired
        else:
            c["mid-paragraph pairs"] += 1
            c["mid-paragraph pairs cut"] += fired


# --- the report ----------------------------------------------------------------------------------


def band(shares):
    if not shares:
        return "-"
    s = sorted(shares)
    return "%s .. %s, median %s" % (pct(s[0], 1), pct(s[-1], 1), pct(s[len(s) // 2], 1))


def at_max(per_page):
    """The pages attaining the maximum share, `page (n of d)`, at most WORST of them."""
    shares = {p: n / d for p, (d, n) in per_page.items() if d}
    if not shares or max(shares.values()) == 0:
        return "none"
    top = max(shares.values())
    pages = sorted(p for p, s in shares.items() if s == top)
    return ", ".join("%d (%d of %d)" % (p, per_page[p][1], per_page[p][0]) for p in pages[:WORST]) + (
        " and %d more" % (len(pages) - WORST) if len(pages) > WORST else "")


def check_line(c):
    return ("%s boundaries, %s found (%s); %s mid-paragraph pairs, %s cut (%s)" % (
        fmt(c["boundaries"]), fmt(c["boundaries found"]),
        pct(c["boundaries found"], c["boundaries"]),
        fmt(c["mid-paragraph pairs"]), fmt(c["mid-paragraph pairs cut"]),
        pct(c["mid-paragraph pairs cut"], c["mid-paragraph pairs"])))


def report(j):
    t = j.tree
    n_elems, n_p, via_map = t.p_summary()
    print("\n%s: %d pages, %s text runs" % (j.name, j.n_pages, fmt(j.n_runs)))
    print("  structure tree: %s elements, %s /P after /RoleMap%s; %s (page, mcid) citations" % (
        fmt(n_elems), fmt(n_p),
        " (%s of them /P only through it: %s)" % (
            fmt(sum(via_map.values())), ", ".join("/%s %d" % kv for kv in via_map.most_common()))
        if via_map else "", fmt(len(t.bindings))))
    for k, v in sorted(t.counts.items()):
        print("    tree: %s: %s" % (k, fmt(v)))
    if j.page_mismatch:
        print("  page record disagrees with native_locator.pdf.page on %d runs" % j.page_mismatch)
    print("  runs excluded: %s" % fmt(sum(j.reasons.values())))
    for k, v in j.reasons.most_common():
        print("    %-50s %s" % (k, fmt(v)))
    print("  runs bound to a /P: %s (through an inline child: %s; to a /P that is /P only through "
          "the /RoleMap: %s runs in %s elements)" % (
              fmt(len(j.bound)), fmt(j.via_inline), fmt(j.via_rolemap[1]), fmt(j.via_rolemap[0])))
    print("  owner role, this walk against the engine's role_path: %s" % ", ".join(
        "%s %s" % (k, fmt(v)) for k, v in j.agree.most_common()))

    m, b = j.share_i()
    nt = len(j.table_blocks)
    print("  (i)  blocks with a /P-bound run: %s; holding 2+ distinct /P: %s (%s)" % (
        fmt(b), fmt(m), pct(m, b)))
    print("       holding a /P outside a /Table: %s, mixed %s (%s); holding only /Table-cell /P: "
          "%s, mixed %s (%s)" % (fmt(b - nt), fmt(m - j.mixed_table), pct(m - j.mixed_table, b - nt),
                                 fmt(nt), fmt(j.mixed_table), pct(j.mixed_table, nt)))
    print("       author boundaries inside mixed blocks (sum of /P - 1): %s, %s of them outside "
          "tables" % (fmt(j.crossed), fmt(j.crossed_prose)))
    print("       mixed blocks the rule declined (block None): %s; numbered: %s" % (
        fmt(j.mixed_none), fmt(m - j.mixed_none)))
    print("       per page: %s; at the maximum: page %s" % (
        band([mx / bs for bs, mx in j.page_blocks.values() if bs]), at_max(j.page_blocks)))
    worst = sorted(j.page_blocks.items(), key=lambda kv: (-kv[1][1], -(kv[1][1] / kv[1][0]), kv[0]))
    for page, (bs, mx) in worst[:WORST]:
        if mx == 0:
            break
        (region, block), k = j.page_worst_block[page]
        print("       page %d: %d of %d blocks mixed, %d /P on the page; its worst block "
              "(region %s, block %s) holds %d /P" % (
                  page, mx, bs, j.page_elems[page][0], region, block, k))
    md, bd = j.share_i_direct()
    print("       direct citations only (probe3b.py's rule): %s of %s (%s)" % (
        fmt(md), fmt(bd), pct(md, bd)))

    s, e = j.share_ii()
    print("  (ii) /P elements with a bound run: %s; in 2+ blocks: %s (%s)" % (fmt(e), fmt(s), pct(s, e)))
    for k, v in j.split_cause.most_common():
        print("       %-36s %s" % (k, fmt(v)))
    print("       of the split elements, %s are inside /Table cells" % fmt(j.split_table))
    print("       per page: %s; at the maximum: page %s" % (
        band([sp / es for es, sp in j.page_elems.values() if es]), at_max(j.page_elems)))
    worst = sorted(j.page_elems.items(), key=lambda kv: (-kv[1][1], -(kv[1][1] / kv[1][0]), kv[0]))
    for page, (es, sp) in worst[:WORST]:
        if sp == 0:
            break
        print("       page %d: %d of %d /P split, %d blocks on the page" % (
            page, sp, es, j.page_blocks[page][0]))
    sd, ed = j.share_ii_direct()
    print("       direct citations only: %s of %s (%s)" % (fmt(sd), fmt(ed), pct(sd, ed)))

    print("  (iv) blocks with a /P-bound run: %s numbered, %s with block None (rule declined); "
          "%s with region None (page not divided)" % (
              fmt(b - j.none_blocks), fmt(j.none_blocks), fmt(j.none_region)))
    lines = sorted(j.lines_per_p)
    if lines:
        print("  lines per /P: mean %.2f, median %d, one-line /P %s" % (
            sum(lines) / len(lines), lines[len(lines) // 2],
            pct(sum(1 for x in lines if x == 1), len(lines))))
    c = j.check
    print("  shipped cut on this instrument's rows: %s; %s digit-only lines dropped, %s bands "
          "failed the guard" % (check_line(c), fmt(c["digit-only lines dropped"]),
                                fmt(c["bands failing the plausibility guard"])))


def summary(joined):
    print("\n%-20s %5s %7s %10s %8s %7s %16s %14s %6s" % (
        "document", "pages", "/P", "bound", "lines/P", "1-line", "(i) mixed", "(ii) split", "None"))
    for j in joined:
        m, b = j.share_i()
        s, e = j.share_ii()
        lines = j.lines_per_p
        print("%-20s %5d %7s %10s %8s %7s %16s %14s %6d" % (
            j.name, j.n_pages, fmt(e), fmt(len(j.bound)),
            "%.2f" % (sum(lines) / len(lines)) if lines else "-",
            pct(sum(1 for x in lines if x == 1), len(lines)),
            "%s/%s %s" % (fmt(m), fmt(b), pct(m, b)),
            "%s/%s %s" % (fmt(s), fmt(e), pct(s, e)), j.none_blocks))
    if len(joined) > 1:
        for label, f in (("(i)", Joined.share_i), ("(ii)", Joined.share_ii)):
            vals = sorted((n / d, j.name) for j in joined for n, d in [f(j)] if d)
            print("%s across documents: %s (%s) .. %s (%s), median %s" % (
                label, pct(vals[0][0], 1), vals[0][1], pct(vals[-1][0], 1), vals[-1][1],
                pct(vals[len(vals) // 2][0], 1)))


def main(argv):
    probe = "--probe3b" in argv
    argv = [a for a in argv if a != "--probe3b"]
    if len(argv) < 2 or len(argv) % 2:
        sys.exit("usage: paragraphs.py [--probe3b] <extract.json> <source.pdf> [...]")
    joined = []
    for i in range(0, len(argv), 2):
        extract, source = argv[i], argv[i + 1]
        name = Path(source).name.replace(".qpdf.json", "").replace(".pdf", "")
        pages, runs = read_extract(extract)
        j = Joined(name, pages, runs, Tree(qpdf_json(source)))
        report(j)
        if probe:
            if source.endswith(".json"):
                print("  --probe3b skipped: no PDF to run qpdf --qdf on")
            else:
                c = j.probe3b_check(extract, source)
                print("  shipped cut on probe3b.py's own rows (%s /P elements structelem.py binds): "
                      "%s" % (fmt(c["/P elements structelem.py binds"]), check_line(c)))
                for k, v in sorted(c.items()):
                    if k.startswith("missed") or k.startswith("found by"):
                        print("    %s: %s" % (k, fmt(v)))
        joined.append(j)
    summary(joined)


if __name__ == "__main__":
    main(sys.argv[1:])
