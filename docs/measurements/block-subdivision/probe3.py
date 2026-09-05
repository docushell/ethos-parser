#!/usr/bin/env python3
"""Probe 3 of `docs/19-BLOCK-SUBDIVISION-SCOPE.md` §7: adaptive versus fixed.

§4.2 found the fixed multiplier fails not because the signal is weak but because each document sets
its break at a near-constant pitch OF ITS OWN — `irs-fw9` at 1.33x its leading, `nist-sp-800-207` at
1.87x — so one constant cleaves the corpus. The rule the evidence points at finds each band's own
SECOND MODE and cuts in the trough beneath it. This tests whether that actually beats a constant.

# The split this probe is forced into, and why it is honest

Probe 1 (§9) left exactly one document with real paragraph labels. But "adaptive beats fixed" is a
claim about variation ACROSS documents, which one document cannot exhibit. So:

  * **Test A — the mechanism, six documents, role-change labels.** Whether adaptive closes the
    28.8%-97.1% spread is a RELATIVE comparison, and both rules are scored against the identical
    labels, so a label set that is a subset of true boundaries (§4.1) is adequate for it. The
    ABSOLUTE numbers remain heading-and-list numbers and are not this feature's recall.
  * **Test B — the magnitude, one document, real P->P labels.** Only `nist-sp-800-207` can supply
    these. Per decision #18 a one-document figure is not a corpus figure and is not published as
    one.

# The rule under test

Per band: bin gaps to 10 centipoints; `leading` is the modal bin. Candidate second modes are bins
above `1.15 x leading` holding at least `max(2, 5%)` of the band's gaps; `second` is the largest.
The cut sits in the trough, midway between them. **No second mode means no cut in that band** — the
rule declines rather than guessing, which is the behaviour §8 rule 4 asks for.

The plausibility guard from §5 excludes bands that are not text flows: modal gap >= 600 centipoints
and the mode holding >= 25% of the band's gaps.
"""
import json
import sys
from collections import Counter, defaultdict

sys.path.insert(0, "/private/tmp/claude-501/-Users-saumildiwaker-Desktop-Stuff-project-repo-ethos-parser/b06430fd-969a-4694-b6d8-a5e5a3044056/scratchpad")
from labelled import lines_with_roles, leading_of  # noqa: E402

MIN_LEAD = 600      # centipoints; below this the "band" is a column interleave, not a text flow
MIN_SHARE = 0.25    # the modal gap must actually dominate


def band_rule(gaps):
    """(leading, cut_threshold_in_centipoints or None, passes_guard)."""
    if len(gaps) < 5:
        return None, None, False
    bins = Counter(round(g / 10) * 10 for g in gaps)
    lead, lead_n = bins.most_common(1)[0]
    if not lead:
        return None, None, False
    guard = lead >= MIN_LEAD and lead_n / len(gaps) >= MIN_SHARE
    floor = 1.15 * lead
    need = max(2, 0.05 * len(gaps))
    cands = [(v, c) for v, c in bins.items() if v > floor and c >= need]
    if not cands:
        return lead, None, guard
    second = max(cands, key=lambda vc: vc[1])[0]
    return lead, (lead + second) / 2.0, guard


def score(pairs):
    """pairs: [(gap, leading, cut, is_boundary)] -> (recall, fire_rate_on_non_boundary)."""
    pos = [p for p in pairs if p[3]]
    neg = [p for p in pairs if not p[3]]
    rec = sum(1 for g, _l, c, _b in pos if c is not None and g > c) / len(pos) if pos else 0.0
    fp = sum(1 for g, _l, c, _b in neg if c is not None and g > c) / len(neg) if neg else 0.0
    return rec, fp


def fixed(pairs, T):
    pos = [p for p in pairs if p[3]]
    neg = [p for p in pairs if not p[3]]
    rec = sum(1 for g, l, _c, _b in pos if g > T * l) / len(pos) if pos else 0.0
    fp = sum(1 for g, l, _c, _b in neg if g > T * l) / len(neg) if neg else 0.0
    return rec, fp


def collect(path, guard_only):
    """[(gap, leading, adaptive_cut, is_declared_boundary)] over one document's bands."""
    doc = json.load(open(path))
    out = []
    bands, _ = lines_with_roles(doc)
    for _key, rows in bands.items():
        gs = [b[0] - a[0] for a, b in zip(rows, rows[1:]) if b[0] > a[0]]
        lead, cut, guard = band_rule(gs)
        if lead is None or (guard_only and not guard):
            continue
        for a, b in zip(rows, rows[1:]):
            g = b[0] - a[0]
            if g <= 0 or g / lead > 5.0:
                continue
            out.append((g, lead, cut, a[1] != b[1]))
    return out


def main(paths):
    print("=== TEST A — mechanism, role-change labels, six documents ===")
    print("    (absolute values are heading/list numbers; the COMPARISON is the finding)\n")
    print(f"{'document':<24}{'fixed 1.6x':>22}{'adaptive':>22}")
    print(f"{'':<24}{'recall / fires':>22}{'recall / fires':>22}")
    alln = []
    for p in paths:
        pairs = collect(p, guard_only=True)
        if not pairs:
            continue
        alln += pairs
        fr, ff = fixed(pairs, 1.6)
        ar, af = score(pairs)
        name = p.split("/")[-1].replace(".json", "")
        print(f"{name:<24}{fr:>11.1%} /{ff:>8.1%}{ar:>11.1%} /{af:>8.1%}")
    fr, ff = fixed(alln, 1.6)
    ar, af = score(alln)
    print(f"\n{'POOLED':<24}{fr:>11.1%} /{ff:>8.1%}{ar:>11.1%} /{af:>8.1%}")

    # the spread is the thing the fixed rule failed on
    def spread(fn):
        vs = []
        for p in paths:
            pairs = collect(p, guard_only=True)
            if pairs and any(x[3] for x in pairs):
                vs.append(fn(pairs)[0])
        return (min(vs), max(vs), max(vs) - min(vs)) if vs else (0, 0, 0)

    lo1, hi1, sp1 = spread(lambda ps: fixed(ps, 1.6))
    lo2, hi2, sp2 = spread(score)
    print(f"\n  per-document recall spread, fixed 1.6x : {lo1:.1%} - {hi1:.1%}  ({sp1*100:.0f} pts)")
    print(f"  per-document recall spread, adaptive   : {lo2:.1%} - {hi2:.1%}  ({sp2*100:.0f} pts)")


if __name__ == "__main__":
    main(sys.argv[1:])
