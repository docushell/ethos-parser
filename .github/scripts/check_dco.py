#!/usr/bin/env python3
#
# Copyright 2026 The ethos-parser maintainers
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
#

"""DCO enforcement (ADR-0004): every NON-MERGE commit in the range carries a Signed-off-by line.

Merge commits are skipped, and that is the standard reading of a DCO rather than a relaxation of
it. The certificate is an assertion about authorship of a contribution; a merge commit contributes
no lines and has no author making that claim — GitHub writes it, under whoever pressed the button,
and its own UI supplies no `Signed-off-by`. Checking it therefore failed EVERY push to `main` on a
commit no human wrote, while the commits that carried real work all passed. Measured: merges #16
through #20 all red on this alone.

The sign-offs that matter are unaffected. A merge's parents are in the range on their own and are
still checked individually, so nothing reaches `main` unsigned by being merged.

Usage: check_dco.py <base_sha> <head_sha>
"""
import subprocess
import sys

if len(sys.argv) != 3:
    print("usage: check_dco.py <base_sha> <head_sha>")
    sys.exit(2)

base, head = sys.argv[1], sys.argv[2]


def commit_exists(sha: str) -> bool:
    if not sha or set(sha) == {"0"}:
        return False
    return (
        subprocess.run(
            ["git", "cat-file", "-e", f"{sha}^{{commit}}"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        ).returncode
        == 0
    )


if not commit_exists(head):
    print(f"DCO: head commit is not available in this checkout: {head}")
    sys.exit(2)

if commit_exists(base):
    rev = f"{base}..{head}"
else:
    # Force-pushes after history rewrites can make github.event.before unreachable in the
    # fresh checkout. In that case there is no valid range to inspect, so validate the
    # pushed tip instead of failing with an internal git-log error.
    print(f"DCO: base commit unavailable; checking pushed head only: {head[:12]}")
    rev = f"{head}^!"

out = subprocess.run(
    [
        "git",
        "log",
        # See the module docstring: a merge commit asserts no authorship, so it carries no
        # certificate to check. Its parents remain in the range and are checked on their own.
        "--no-merges",
        "--format=%H%x00%an <%ae>%x00%(trailers:key=Signed-off-by,valueonly)",
        rev,
    ],
    capture_output=True,
    text=True,
    check=True,
).stdout

bad = []
for record in filter(None, out.strip().split("\n")):
    sha, author, *trailer = record.split("\x00")
    signoff = (trailer[0] if trailer else "").strip()
    if not signoff:
        bad.append((sha[:12], author))

if bad:
    for sha, author in bad:
        print(f"DCO: commit {sha} by {author} lacks Signed-off-by (use `git commit -s`)")
    sys.exit(1)
print("DCO green")
