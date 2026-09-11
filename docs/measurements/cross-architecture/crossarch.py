#!/usr/bin/env python3
"""Do two CPU architectures produce byte-identical artifacts? (corrected instrument)

The first version of this ran `markdown` and `html` against PDFs. They take a DocumentRepresentation
JSON, not a PDF, so 194 of 344 comparisons were the two subcommands correctly REFUSING their input.
That still compares something — a refusal message is a result and must be identical too — but it is
not the projection comparison it claimed to be, and reporting it as one would have overclaimed.

So: PDF-taking subcommands get the PDF; representation-taking subcommands get a representation,
produced once by the native binary and handed to BOTH arms, so this measures the projection rather
than re-measuring `extract`.

6.1 asks for three-OS byte-identity and is blocked on runners with no cross toolchain here. This is
the axis that needs neither: an arm64 Mac with Rosetta 2 can build and EXECUTE an x86_64 binary.
Different instruction set, different codegen, same pinned 1.88.0. It is the axis the design claims
to have settled by construction — integer centipoints exist so geometry cannot vary with a
machine's floating-point unit — so a disagreement here would be worse news than one across
operating systems.
"""
import hashlib, pathlib, subprocess, sys, tempfile

ARM = "target/release/ethos-parser"
X86 = "target/x86_64-apple-darwin/release/ethos-parser"
FROM_PDF = ["extract", "classify"]
FROM_REPR = ["markdown", "html", "ground"]
# The two largest gate documents are excluded from the representation-taking subcommands only:
# 53Ar5's representation is 950 MB and projecting it twice per arm is hours, not minutes. Named
# here rather than silently dropped.
TOO_BIG = {"nist-sp-800-53Ar5", "nist-sp-800-161r1"}


def run(binary, sub, path):
    d = subprocess.run([binary, sub, str(path)], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if d.returncode != 0:
        return f"exit{d.returncode}:" + hashlib.sha256(d.stderr).hexdigest()[:16], None
    return hashlib.sha256(d.stdout).hexdigest(), d.stdout


docs = sorted(pathlib.Path("fixtures/gate").glob("*.pdf"))
docs += sorted(pathlib.Path("fixtures/engine").rglob("*.pdf"))
conf = pathlib.Path("../ethos/fixtures")
if conf.is_dir():
    docs += sorted(conf.rglob("*.pdf"))

mismatches = []
n_pdf = n_repr = n_refusal = 0

with tempfile.TemporaryDirectory() as tmp:
    for doc in docs:
        for sub in FROM_PDF:
            a, out = run(ARM, sub, doc)
            b, _ = run(X86, sub, doc)
            n_pdf += 1
            if a.startswith("exit"):
                n_refusal += 1
            if a != b:
                mismatches.append((str(doc), sub, a, b))
                print(f"  *** MISMATCH {sub} {doc}", flush=True)

        if doc.stem in TOO_BIG:
            continue
        # One representation, produced by the native arm, fed to both arms.
        a, repr_bytes = run(ARM, "extract", doc)
        if repr_bytes is None:
            continue
        rp = pathlib.Path(tmp) / f"{doc.stem}.json"
        rp.write_bytes(repr_bytes)
        for sub in FROM_REPR:
            a, _ = run(ARM, sub, rp)
            b, _ = run(X86, sub, rp)
            n_repr += 1
            if a.startswith("exit"):
                n_refusal += 1
            if a != b:
                mismatches.append((str(doc), sub, a, b))
                print(f"  *** MISMATCH {sub} {doc}", flush=True)

print(f"\ndocuments:              {len(docs)}")
print(f"PDF-taking comparisons:  {n_pdf}   (extract, classify)")
print(f"representation-taking:   {n_repr}   (markdown, html, ground)")
print(f"  of which refusals:     {n_refusal}  (compared as refusals — identical stderr required)")
print(f"excluded from projection: {sorted(TOO_BIG)} — representation too large to project twice per arm")
print(f"mismatches:              {len(mismatches)}")
print("\nBYTE-IDENTICAL ACROSS ARCHITECTURES" if not mismatches else "\n*** ARCHITECTURE-DEPENDENT ***")
sys.exit(1 if mismatches else 0)
