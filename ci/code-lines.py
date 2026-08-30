#!/usr/bin/env python3
"""Emit the production code corpus of `crates/*/src/**.rs` — every non-comment line outside
`mod tests { … }` — so that two trees can be diffed for behaviour change.

    ci/code-lines.py                    # the working tree
    ci/code-lines.py --rev HEAD         # a git revision, read without touching the worktree
    ci/code-lines.py --self-test        # the negative control alone, then exit

    diff <(ci/code-lines.py --rev HEAD) <(ci/code-lines.py)   # the no-behaviour-change proof

Every patch slice since v2-S9.1 has an acceptance box reading "no behaviour change, proven
mechanically", and until v2-S16 each slice rebuilt this extractor by hand. Four were built and
**three were wrong**, each in a way the others could not see, and the record carries four
different absolute counts for one rule. This file exists so there is one instrument to disagree
with rather than four to reconcile. `docs/history/15-V2-MILESTONES.md` S15's correction notes derive it.

# One pass, two line-aligned renderings, and neither alone is sufficient

The pass produces two strings of identical length for every file:

  * a **skeleton**, with the *contents* of every string, raw-string, byte-string and character
    literal replaced by spaces. Used to find `//`, to find `mod tests {`, and to find the `}`
    that closes it.
  * a **display**, the source unchanged. Used for the emitted line, so that a change *inside* a
    literal is visible.

v2-S14.1's extractor emitted the skeleton and was therefore **blind to every wire string**: it
reported an empty diff for a slice that changed two emitted messages. v2-S15's extractor emitted
the display but also matched braces on it, so a `{` inside a string literal broke the `mod tests`
matching and test code leaked into the corpus — S15's own correction records 186 `assert` lines
and 78 `#[test]` attributes reaching it. Blanking
the literals makes the proof blind; preserving them without a separate skeleton makes it leak.
Both defects were invisible until the other was corrected.

# The test-module rule is the repository's own, not a second opinion

`ci/forbidden-tokens.sh` line 82 is `/^mod tests \{/` — anchored at column 0, with no
`pub(crate)` alternative — and its skip resumes at the matching `^}` rather than running to end
of file. This file matches that rule exactly, because a second definition of "test module" in one
repository is the drift a shared rule exists to prevent.

`crates/ethos-parser-core/src/markdown.rs` is the **one** file in the tree that writes
`pub(crate) mod tests {`; fifty-two others write `mod tests {`. The anchored rule therefore scans
markdown.rs's test module and a looser one would not, a difference of 1,017 lines — all in that
single file. The anchored rule is right on the repository's own authority and on
`forbidden-tokens.sh`'s stated direction of safety, "a false alarm is cheap next to a missed
one": including a test module errs toward reporting.

# What the number is not

The absolute count is an implementation artifact — of choices about blank lines, braces and
module boundaries — and not a property of the tree. Quoting it as a measurement is what invited
three separate chases across v2-S13.5 through v2-S15. **The invariant is the diff under one
instrument held fixed across both sides.** Quote the diff.
"""

import argparse
import os
import re
import subprocess
import sys

IDENT = re.compile(r"[A-Za-z0-9_]")

# The guard's own rules, spelled the same way: anchored at column 0, no `pub(crate)` alternative.
MOD_TESTS_OPEN = re.compile(r"^mod tests \{")
MOD_TESTS_CLOSE = re.compile(r"^\}")


class BlockComment(Exception):
    """A `/* */` appeared in code context.

    `ci/forbidden-tokens.sh` hard-fails on the same construct for the same reason: `//`-stripping
    is only safe because no block comment can hide a line inside one. Refusing loudly is the only
    honest answer — an instrument that silently mis-tokenizes measures nothing.
    """


def render(text):
    """Return (skeleton, display) — two strings of identical length.

    The skeleton replaces literal *contents* with spaces and keeps the delimiters, so column
    positions and line boundaries are preserved and the two renderings stay line-aligned.

    Literal state is carried **across line boundaries**: there is no per-line reset, so a
    multi-line string containing `//` survives intact rather than truncating at it.
    """
    out = []
    i = 0
    n = len(text)
    while i < n:
        c = text[i]
        prev = text[i - 1] if i else ""

        # A line comment. Copied through unchanged — the skeleton blanks *literals*, and blanking
        # a comment as well would destroy the very `//` that strip_comment() has to find. It is
        # consumed here rather than scanned so that a quote inside a comment cannot open a string.
        if c == "/" and i + 1 < n and text[i + 1] == "/":
            j = text.find("\n", i)
            j = n if j < 0 else j
            out.append(text[i:j])
            i = j
            continue

        if c == "/" and i + 1 < n and text[i + 1] == "*":
            line = text.count("\n", 0, i) + 1
            raise BlockComment(f"line {line}")

        # A raw string: r"…", r#"…"#, br##"…"##. `r#type` is a raw *identifier*, not a string, so
        # the `#`-run must be followed by a quote for this to be a literal at all.
        if c in "rb" and not IDENT.match(prev or " "):
            j = i
            if text[j] == "b" and j + 1 < n and text[j + 1] == "r":
                j += 1
            if text[j] == "r":
                k = j + 1
                while k < n and text[k] == "#":
                    k += 1
                if k < n and text[k] == '"':
                    hashes = k - (j + 1)
                    close = '"' + "#" * hashes
                    end = text.find(close, k + 1)
                    end = n if end < 0 else end + len(close)
                    out.append(text[i : k + 1])
                    out.append(blank(text[k + 1 : end - len(close)]))
                    out.append(text[end - len(close) : end])
                    i = end
                    continue

        # A byte string, b"…", is an ordinary string literal for this purpose.
        if c == "b" and i + 1 < n and text[i + 1] == '"' and not IDENT.match(prev or " "):
            out.append("b")
            i += 1
            c = text[i]

        if c == '"':
            j = i + 1
            while j < n:
                if text[j] == "\\":
                    j += 2
                    continue
                if text[j] == '"':
                    break
                j += 1
            j = min(j, n)
            out.append('"')
            out.append(blank(text[i + 1 : j]))
            if j < n:
                out.append('"')
            i = j + 1
            continue

        # A `'` is a character literal only if it closes. Otherwise it is a lifetime or a loop
        # label, and treating `'static` as an unterminated string is how six phantom literals
        # were once opened by the byte literal `b'"'` in `c14n.rs`.
        if c == "'":
            end = char_literal_end(text, i)
            if end is not None:
                out.append("'")
                out.append(blank(text[i + 1 : end]))
                out.append("'")
                i = end + 1
                continue

        out.append(c)
        i += 1

    skeleton = "".join(out)
    assert len(skeleton) == n, "the skeleton must stay byte-aligned with the source"
    return skeleton, text


def blank(s):
    """Replace a literal's contents with spaces, keeping newlines so the line count is preserved."""
    return "".join("\n" if ch == "\n" else " " for ch in s)


def char_literal_end(text, start):
    """Index of the closing `'` of a character literal at `start`, or None for a lifetime."""
    n = len(text)
    i = start + 1
    if i >= n:
        return None
    if text[i] == "\\":
        i += 1
        if i < n and text[i] == "u":
            close = text.find("}", i)
            if close < 0:
                return None
            i = close
        i += 1
        return i if i < n and text[i] == "'" else None
    return i + 1 if i + 1 < n and text[i + 1] == "'" else None


def strip_comment(skeleton_line, display_line):
    """Cut both renderings at the `//` the skeleton found, with its leading whitespace.

    The same substitution `ci/forbidden-tokens.sh` line 85 applies — `[ \\t]*//.*$` — except that
    the position comes from the skeleton, so a `//` inside a string literal is not a comment.
    """
    cut = skeleton_line.find("//")
    if cut < 0:
        return display_line
    while cut > 0 and display_line[cut - 1] in " \t":
        cut -= 1
    return display_line[:cut]


def corpus(files):
    """Emit `path\\tline` for every non-comment line outside `mod tests { … }`."""
    lines = []
    for path, text in files:
        skeleton, display = render(text)
        in_tests = False
        for sk, dp in zip(skeleton.split("\n"), display.split("\n")):
            if not in_tests and MOD_TESTS_OPEN.match(sk):
                in_tests = True
                continue
            if in_tests:
                if MOD_TESTS_CLOSE.match(sk):
                    in_tests = False
                continue
            kept = strip_comment(sk, dp)
            if kept.strip(" \t"):
                lines.append(f"{path}\t{kept}")
    return lines


def is_source(path):
    return path.endswith(".rs") and "/src/" in path and path.startswith("crates/")


def worktree_files(root):
    paths = []
    for dirpath, _dirnames, filenames in os.walk(os.path.join(root, "crates")):
        for name in filenames:
            rel = os.path.relpath(os.path.join(dirpath, name), root)
            if is_source(rel):
                paths.append(rel)
    for rel in sorted(paths):
        with open(os.path.join(root, rel), encoding="utf-8") as fh:
            yield rel, fh.read()


def revision_files(root, rev):
    listing = subprocess.run(
        ["git", "-C", root, "ls-tree", "-r", "--name-only", rev, "--", "crates"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.splitlines()
    for rel in sorted(p for p in listing if is_source(p)):
        blob = subprocess.run(
            ["git", "-C", root, "show", f"{rev}:{rel}"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout
        yield rel, blob


# -------------------------------------------------------------------------------------------
# The negative control
# -------------------------------------------------------------------------------------------
#
# It runs on **every** invocation, not only under `--self-test`, because an extractor that cannot
# detect a change measures nothing and the moment that matters is the moment it is used. Every
# earlier slice negative-controlled its extractor in a transcript and threw the control away with
# the script; this one ships with the thing it controls, in the same file and the same language.
#
# The cost is stated rather than hidden: this control has no `cargo test` home, so a break in this
# file is not reported by the gate suite. It is reported the next time anyone runs the instrument,
# which is the only run whose soundness the proof depends on.

CONTROL_SOURCE = '''use std::fmt;

pub const KEEP: usize = 1;

/// A doc comment.
pub fn emit() -> &'static str {
    "a header this engine never read: no detector reads /TH"
}

pub const HELP: &str = "usage:
mod tests {
see http://example.invalid for more
";

pub const TAIL: usize = 2;

mod tests {
    use super::*;

    const NOTE: &str = "a fixture whose next line opens at column zero:
}
";

    #[test]
    fn a_test() {
        assert_eq!(KEEP, 1, "LEAKED");
    }
}
'''

# Each axis is (name, source, must appear in the corpus, must not appear in it). The first four
# are the axes every earlier slice controlled and threw away; the last three are the ones that
# make "neither rendering alone is sufficient" a checked claim rather than a sentence.
#
# Axes 5 and 6 both break under one mistake — matching the module rule on the display instead of
# the skeleton — and they break in opposite directions, which is why both are here. Axis 5 is the
# dangerous one: an extractor that stops reporting is a proof that always passes.
CONTROL_AXES = [
    (
        "a `pub const` outside `mod tests` is reported",
        CONTROL_SOURCE.replace(
            "pub const KEEP: usize = 1;",
            "pub const KEEP: usize = 1;\npub const INJECTED: usize = 3;",
        ),
        ["INJECTED"],
        [],
    ),
    (
        "a `//` comment is ignored",
        CONTROL_SOURCE.replace(
            "pub const KEEP: usize = 1;",
            "// INJECTED\npub const KEEP: usize = 1;",
        ),
        [],
        ["INJECTED"],
    ),
    (
        "a line inside `mod tests` is ignored",
        CONTROL_SOURCE.replace(
            "        assert_eq!(KEEP, 1, \"LEAKED\");",
            "        assert_eq!(KEEP, 1, \"LEAKED\");\n        let _ = INJECTED;",
        ),
        [],
        ["INJECTED"],
    ),
    (
        "a word injected into an emitted wire string is reported",
        CONTROL_SOURCE.replace(
            "no detector reads /TH",
            "no INJECTED detector reads /TH",
        ),
        ["INJECTED"],
        [],
    ),
    (
        "`mod tests {` inside a string literal does not start the skip",
        CONTROL_SOURCE,
        ["TAIL"],
        [],
    ),
    (
        "a column-zero `}` inside a string literal does not end the skip",
        CONTROL_SOURCE,
        [],
        ["LEAKED"],
    ),
    (
        "a `//` inside a multi-line string does not truncate the line",
        CONTROL_SOURCE,
        ["example.invalid"],
        [],
    ),
]


def self_test():
    """Run the negative control. An extractor that cannot detect a change measures nothing."""
    failures = []
    for name, source, expected, forbidden in CONTROL_AXES:
        text = "\n".join(corpus([("control.rs", source)]))
        for want in expected:
            if want not in text:
                failures.append(f"  {name}: `{want}` is missing from the corpus")
        for reject in forbidden:
            if reject in text:
                failures.append(f"  {name}: `{reject}` reached the corpus")
    if failures:
        raise SystemExit(
            "::error::ci/code-lines.py failed its own negative control:\n"
            + "\n".join(failures)
            + "\n\nFix this file before trusting any diff it produced."
        )


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--rev", help="read the sources at a git revision instead of the worktree")
    ap.add_argument(
        "--self-test",
        action="store_true",
        help="run the negative control and exit, emitting no corpus",
    )
    args = ap.parse_args()

    self_test()
    if args.self_test:
        print(f"control: {len(CONTROL_AXES)} axes pass.", file=sys.stderr)
        return 0

    root = subprocess.run(
        ["git", "rev-parse", "--show-toplevel"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()

    files = list(revision_files(root, args.rev) if args.rev else worktree_files(root))
    if not files:
        print("::error::no crate sources found; the scan would pass vacuously", file=sys.stderr)
        return 1

    try:
        lines = corpus(files)
    except BlockComment as exc:
        print(
            f"::error::a /* */ block comment appeared in crates/*/src ({exc}); this instrument "
            "only strips // comments, and ci/forbidden-tokens.sh refuses the same construct",
            file=sys.stderr,
        )
        return 1

    sys.stdout.write("".join(line + "\n" for line in lines))
    print(
        f"{len(lines)} lines from {len(files)} files"
        + (f" at {args.rev}" if args.rev else " in the working tree"),
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
