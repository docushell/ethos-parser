#!/usr/bin/env python3
"""Differential: `grounding-check` on mutated artifacts, one binary against another, byte for byte.

A change that must not move any report is checked where the reports are most varied: on broken
input. Each seed artifact is mutated deterministically — a number swapped for a float, a huge or a
negative integer, null, or a wrong type; a string value for null, a long string at and past the
16,384-byte limit, a lone surrogate, or nesting at and past depth 64; a key inserted, unknown or
repeated; a strict fault and a key together; a key dropped; raw bytes deleted, inserted or replaced;
a root member nulled or retyped; a BOM, trailing bytes, an empty file. Both binaries check every
mutant, and exit code, stdout and stderr must all be equal. Prints the count that differ and the
distribution of report codes, so a run that exercised only one code is visible.
Usage: gcdiff.py OLD NEW SCRATCH_DIR seed.json ...
"""
import collections, concurrent.futures as cf, json, os, random, re, subprocess, sys, tempfile

OLD, NEW, OUT = sys.argv[1], sys.argv[2], sys.argv[3]
SEEDS = sys.argv[4:]
rng = random.Random(20260914)

NUMBER = re.compile(rb'(?<=[:\[,])-?\d+(?=[,\]}])')
STRING_VALUE = re.compile(rb'(?<=:)"(?:[^"\\]|\\.)*"')
OBJECT_OPEN = re.compile(rb'\{')
KEY = re.compile(rb'"([a-z_]+)":')

NUMBER_SWAPS = [b"0.5", b"-0", b"1e3", b"1.0", b"9007199254740991", b"-9007199254740991", b"9007199254740992",
                b"-9007199254740992", b"-9223372036854775808", b"9223372036854775807", b"18446744073709551615",
                b"18446744073709551616", b"-18446744073709551616", b"null", b'"7"', b"true", b"[]", b"{}"]
VALUE_SWAPS = [b"null", b'""', b'"' + b"a" * 16384 + b'"', b'"' + b"a" * 16385 + b'"',
               b'"' + "é".encode() * 8192 + b'"', b'"' + "é".encode() * 8193 + b'"', b'"\\ud800"', b'"\\u0000"',
               b"[" * 63 + b"]" * 63, b"[" * 64 + b"]" * 64, b"[" * 65 + b"]" * 65, b"[" * 130 + b"]" * 130,
               b'{"a":' * 64 + b"1" + b"}" * 64, b"0.5", b"123", b"false", b"[null]"]
KEY_INSERTS = [b'"zzz":1,', b'"zzz":null,', b'"zzz":0.5,', b'"id":"x",', b'"page":"p1",', b'"text":null,',
               b'"locator":"l",', b'"char_start":0,', b'"cells":[],', b'"zzz":' + b"[" * 70 + b"]" * 70 + b","]


def at(regex, data):
    ms = list(regex.finditer(data))
    return rng.choice(ms) if ms else None


def mutations(data):
    for _ in range(250):
        m = at(NUMBER, data)
        if m:
            yield data[:m.start()] + rng.choice(NUMBER_SWAPS) + data[m.end():]
    for _ in range(250):
        m = at(STRING_VALUE, data)
        if m:
            yield data[:m.start()] + rng.choice(VALUE_SWAPS) + data[m.end():]
    for _ in range(250):
        m = at(OBJECT_OPEN, data)
        yield data[:m.end()] + rng.choice(KEY_INSERTS) + data[m.end():]
    for _ in range(120):  # a strict fault and an unknown key at once, in either order
        a, b = at(NUMBER, data), at(OBJECT_OPEN, data)
        if a and b:
            ins, swap = rng.choice(KEY_INSERTS), rng.choice(NUMBER_SWAPS)
            edits = sorted([(a.start(), a.end(), swap), (b.end(), b.end(), ins)], reverse=True)
            out = data
            for s, e, r in edits:
                out = out[:s] + r + out[e:]
            yield out
    for _ in range(120):  # drop a key and its value, crudely
        m = at(KEY, data)
        if m:
            end = data.find(b",", m.end())
            yield data[:m.start()] + data[end + 1:] if end > 0 else data[:m.start()]
    for _ in range(200):  # raw bytes
        i = rng.randrange(len(data))
        op = rng.randrange(3)
        yield (data[:i] + data[i + 1:]) if op == 0 else (data[:i] + bytes([rng.randrange(256)]) + data[i:]) if op == 1 \
            else (data[:i] + bytes([rng.randrange(256)]) + data[i + 1:])
    obj = json.loads(data)
    for k in ["spans", "tables", "elements", "pages", "source", "producer"]:
        for v in [None, [], {}, 0.5, "x"]:
            o = dict(obj); o[k] = v
            yield json.dumps(o, separators=(",", ":")).encode()
    yield data + b" junk"
    yield data + b"\n"
    yield b"\xef\xbb\xbf" + data
    yield data + data
    yield b""
    yield b"null"
    yield b"[]"


def run(binary, path):
    p = subprocess.run([binary, "grounding-check", path], capture_output=True)
    return p.returncode, p.stdout, p.stderr


def compare(path):
    return path, run(OLD, path), run(NEW, path)


cases = []
for seed in SEEDS:
    data = open(seed, "rb").read()
    base = os.path.basename(seed)
    for i, m in enumerate(mutations(data)):
        p = os.path.join(OUT, f"{base}.{i}.json")
        open(p, "wb").write(m)
        cases.append(p)

codes, differ = collections.Counter(), []
with cf.ThreadPoolExecutor(8) as pool:
    for path, old, new in pool.map(compare, cases):
        if old != new:
            differ.append(path)
        try:
            codes[json.loads(old[1]).get("error", {}).get("code", "valid")] += 1
        except Exception:
            codes[f"exit {old[0]}"] += 1

print(f"{len(cases)} inputs, {len(differ)} differ (exit, stdout or stderr)")
print(dict(codes.most_common()))
for d in differ[:10]:
    print("DIFFER", d)
