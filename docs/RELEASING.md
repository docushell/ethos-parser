# Releasing

**The first release is v0.55.0, and it is binaries only**: a GitHub Release carrying macOS builds —
`aarch64` and `x86_64`, each executed and byte-compared by `ci/release-artifacts.sh` — on a private
repository, following §8. **Nothing is on crates.io, npm or PyPI**, the `publish = false` tripwire in
§6 is still in place, and there is no publish automation. A local `v0.54.0` tag predates this
procedure being followed; it was never pushed and nothing was released from it. `CHANGELOG.md` has said so from its
third line: *"Version numbers are in-tree; creating a tag or a release is a separate, deliberate
act."*

This document is that act, written down before it is performed rather than after — which is the
only useful time to write it, because **publishing is the one thing this repository does that
cannot be undone.**

---

## 1. What is irreversible, and what is not

| | Reversible? |
| --- | --- |
| A version bump in the tree | yes — it is a commit |
| A git tag | yes — `git tag -d` and a force-push, while nobody has fetched it |
| A GitHub Release with binaries | yes — `gh release delete` and delete the tag, while nobody depends on it; on a private repository only collaborators ever saw it |
| **A crates.io publish** | **no.** `cargo yank` stops *new* dependents resolving it; the version number is spent forever and the files stay downloadable |
| **An npm publish** | **no**, in practice. Unpublish is allowed for 72 hours and only if nothing depends on it; after that, `deprecate` |
| **A PyPI publish** | **no.** A deleted file's version can never be reused |

**A first publish also claims a name.** `ethos-parser` on crates.io, npm and PyPI would be taken by
whoever runs step 5 first, and taken permanently.

## 2. What already guards itself, and must not be re-checked here

Three of the four things a release must get right are already tests, and they run in
`ci/gate.sh`. **This procedure does not re-implement them** — a second copy of a check is how the
things in `docs/00-NORTH-STAR.md` row 10 and `docs/table-gate-v1.md` §2 went stale:

| Invariant | Guarded by |
| --- | --- |
| The four version strings agree — `Cargo.toml`, `packages/node/package.json`, `packages/node/src/index.js`, `packages/python/src/ethos_parser/__init__.py` | `sdk_versions.rs::every_sdk_version_is_the_workspace_version` |
| `profile_sha256` matches the profile this build actually produces | `profile.rs::the_default_profile_is_pinned` |
| The local gate runs what CI runs | `v0_exit_criteria.rs::the_local_gate_runs_what_ci_runs` |

**Why the first one exists is worth remembering while releasing.** Those three SDK version guards
were written, were never executed by any workflow, and the numbers drifted **six minor versions** —
`0.36.1` sitting in a `0.42.0` tree — until v2-S15 found it. They run now. A release is exactly the
moment they matter.

The fourth thing — that `CHANGELOG.md` has an entry for the version being released — was guarded by
nothing, which is how 0.42.1 shipped twenty-one commits and described one. `ci/release-preflight.sh`
checks it.

## 3. Before anything: `ci/release-preflight.sh`

```bash
ci/release-preflight.sh
```

It refuses, loudly, on any of: a dirty working tree, a `CHANGELOG.md` with no `## [VERSION]` heading
for the version in `Cargo.toml`, a git tag that already exists for it, or a red gate. It checks only
what nothing else checks and delegates the rest to `ci/gate.sh`, which is where the three guards in
§2 live.

**It is not a substitute for reading this document.** It cannot check that the version number is the
*right* one, and getting that wrong is the failure §4 is about.

## 4. Choosing the number

`docs/01-CONTRACT.md` §2 and the `[workspace.package]` narrative in `Cargo.toml` carry the rule this
repository actually uses, which is stricter than semver's:

- **MINOR when a reader or an emitter changes** — when this build produces different bytes than the
  last one for the same input, or reads an input the last one refused. Not only when an API changes.
- **PATCH only when output is byte-identical at equal version.**

**0.42.1 got this wrong**, and its CHANGELOG entry says so: it was labelled a PATCH on the strength
of the slice it was named for, while the version it was attached to also carried `--max-pages` (a
feature), a ZIP end-of-central-directory repair that widens the archives the engine accepts, and a
font-cache repair that changes which documents produce an artifact at all. **The label describes the
tree, not the commit you happen to be thinking about.** Read `git log <last release>..HEAD` before
choosing, not just the top commit.

Every version moves `profile_sha256`, because `parser_version` is a profile field. That is the
mechanism working: two builds are correctly non-comparable even when nothing else changed.

## 5. The sequence

**5.1 — Land the version bump.** Four version strings, the `CHANGELOG.md` entry, the
`[workspace.package]` narrative clause, and the re-pinned `profile_sha256`. This is a normal PR and
goes through the gate like any other.

**5.2 — Preflight and tag.** On a clean `main` at the merge commit:

```bash
ci/release-preflight.sh
git tag -a v0.43.0 -m "0.43.0"
```

Do **not** push the tag yet. A tag nobody has fetched is still deletable; a published crate is not.

**5.3 — Dry-run every crate, in dependency order.** The order is forced — crates.io will not accept
a crate whose path dependencies are not yet published at the version it names:

```
ethos-parser-core                          # depends on nothing in-tree
  ethos-parser-pdf                         # -> core
  ethos-parser-office                      # -> core
  ethos-parser-grounding                   # -> core
    ethos-parser-cli                       # -> core, pdf, office, grounding
```

```bash
for c in core pdf office grounding cli; do
  cargo publish -p "ethos-parser-$c" --dry-run || break
done
```

A dry run of a crate whose dependencies are unpublished will fail on the version requirement. That
is expected on a first release and is the reason 5.4 goes one at a time.

**5.4 — Remove the tripwire, then publish one crate at a time, verifying between.** `publish = false` in `[workspace.package]` (§6) must be flipped first, in a reviewed commit that does nothing else — it exists so that this step cannot be reached by accident. After each, wait for the index and check
the next crate's dry run passes before continuing. **This is the irreversible step**; everything
above can be abandoned without consequence and nothing below can.

**5.5 — The SDKs.** `packages/node` (`npm publish`) and `packages/python` (build and `twine
upload`). Their version strings are already the workspace version — §2's first guard is what
ensures it — but note that `@langchain/core` is a **devDependency** here and an *optional peer* for
consumers, so a published package must pull nothing on a default install.
`the_default_import_does_not_reach_langchain` asserts that and runs in CI as of 0.43.0.

**5.6 — Push the tag**, last, once everything that can fail has.

## 6. There is no publish automation, deliberately

Nothing in `.github/workflows/` publishes anything, and this document does not ask for that to
change. A workflow that can publish is a workflow that can publish *by accident*, and the failure
mode is the one §1 describes as permanent. The steps above are slow on purpose.

**An accidental `cargo publish` is refused.** `[workspace.package]` carries `publish = false` and
each of the five crate manifests inherits it, so every crate answers:

```
error: `ethos-parser-core` cannot be published.
`package.publish` must be set to `true` or a non-empty list in Cargo.toml to publish.
```

That was added the moment the owner asked for it and not before, because it is a policy about
publishability rather than a procedure. **Removing it is step 5.4's first action** — a tracked edit
somebody reviews, at the moment of release, rather than a sentence in a changelog that nothing
enforces. The friction is the point.

## 7. If a release goes wrong

- **Before 5.4** — delete the tag, fix, start again. Nothing has left the machine.
- **After a bad crates.io publish** — `cargo yank --version X.Y.Z`. New dependents cannot resolve it;
  existing ones are unaffected and the files remain downloadable. **Then release a fixed version.**
  There is no other repair, and the version number is spent.
- **A partially published workspace** — some crates up, some not — is the likeliest bad state,
  because 5.4 is a loop that can stop in the middle. It is recoverable: publish the rest, or yank
  what went out and move to the next patch version. Record which, in `CHANGELOG.md`, because the
  gap will otherwise be visible on crates.io and explained nowhere.

## 8. A binaries-only GitHub Release

The registries in §5.3–5.5 claim names permanently and stay blocked until the owner decides they
should not. A GitHub Release of prebuilt binaries claims nothing and can be deleted, so it can ship
first — and 0.55.0 did. It uses §5.1 and §5.2 unchanged, then:

1. **Build and verify the artifacts** on a clean tree at the tagged commit:

   ```bash
   ci/release-artifacts.sh
   ```

   Every target lands as `verified` — built, executed on this host, and every artifact digest over
   the gate corpus equal to the native build's — or `compiled`, built and never run.
   `target/release-artifacts/SHA256SUMS.txt` records which. **Ship only `verified` targets.** A
   compiled-only binary is an untested claim for an engine whose product is byte-identical reruns;
   Linux and Windows wait for a runner that can execute them (plan item 6.1).

2. **Push the tag**, then create the release from those files and nothing else:

   ```bash
   git push origin v0.55.0
   gh release create v0.55.0 --title "ethos-parser 0.55.0" --notes-file <notes> \
     target/release-artifacts/*.tar.gz target/release-artifacts/SHA256SUMS.txt
   ```

   The notes say which platforms were verified and which were not built, so no reader infers a
   platform from its absence.

**Undoing it**, if something is wrong: `gh release delete v0.55.0`, then `git push --delete origin
v0.55.0` and `git tag -d v0.55.0`. Fix, and release again under the same number only if nobody
outside the repository could have fetched it; otherwise the number is spent, as in §7.
