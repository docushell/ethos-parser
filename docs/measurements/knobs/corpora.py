"""The five PDF corpora the knob measurements run over, as (corpus, path) pairs.

Usage as a script: `corpora.py` prints one `corpus<TAB>path` line per PDF. As a module, `pdfs()`
returns the same list. Roots come from the environment, with defaults relative to this repository:

  ETHOS_PARSER_REPO   this repository (default: three directories above this file)
  ETHOS_ORACLE        the ethos-oracle checkout (default: ../ethos-oracle beside the repository)
  ODL_BENCH_PDFS      the opendataloader-bench pdfs directory
                      (default: ~/ethos-external-benchmarks/opendataloader-bench/pdfs)

A corpus whose root is absent is reported on stderr and skipped, so a partial run says which
corpora it covered rather than silently covering fewer.
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.environ.get("ETHOS_PARSER_REPO") or os.path.abspath(os.path.join(HERE, "..", "..", ".."))
ORACLE = os.environ.get("ETHOS_ORACLE") or os.path.join(os.path.dirname(REPO), "ethos-oracle")
ODL = os.environ.get("ODL_BENCH_PDFS") or os.path.expanduser(
    "~/ethos-external-benchmarks/opendataloader-bench/pdfs"
)

# Name, root directory, and whether to walk it recursively.
CORPORA = [
    ("gate", os.path.join(REPO, "fixtures", "gate"), False),
    ("engine", os.path.join(REPO, "fixtures", "engine"), True),
    ("oracle", os.path.join(ORACLE, "fixtures"), True),
    ("gate-zero", os.path.join(ORACLE, "benchmarks", "gate-zero", "corpus"), False),
    ("odl-bench", ODL, False),
]


def pdfs(only=None):
    """(corpus, absolute path) for every PDF, sorted by path within each corpus."""
    out = []
    for name, root, recurse in CORPORA:
        if only and name not in only:
            continue
        if not os.path.isdir(root):
            print("corpora: %s root absent, skipped: %s" % (name, root), file=sys.stderr)
            continue
        found = []
        if recurse:
            for d, _, files in os.walk(root):
                found.extend(os.path.join(d, f) for f in files if f.lower().endswith(".pdf"))
        else:
            found = [os.path.join(root, f) for f in os.listdir(root) if f.lower().endswith(".pdf")]
        out.extend((name, p) for p in sorted(found))
    return out


def label(corpus, path):
    """A short document label: the path relative to the corpus root."""
    root = dict((n, r) for n, r, _ in CORPORA)[corpus]
    return os.path.relpath(path, root)


if __name__ == "__main__":
    for corpus, path in pdfs(sys.argv[1:] or None):
        print("%s\t%s" % (corpus, path))
