"""Stratified random sample of depth-3 positions for `hib isocost`.
Strata: value pair (standard, redundant-hint) for the player to move. Only positions with
no redundant hint available (p = 0) are used, so both rules start from the same position.
Usage: python3 tools/analysis/iso_sample.py <normal depth3-results> <pass depth3-results> <per stratum> [seed]
Prints lines "key stones stratum".
"""
import random, sys
std, rh = {}, {}
for l in open(sys.argv[1]):
    k, v, _, _, _, _, stones = l.split()
    std.setdefault(k, (int(v), stones))
for l in open(sys.argv[2]):
    k, v, *_ = l.split()
    key, p = k.split(':')
    if p == '0': rh[key] = int(v)
per = int(sys.argv[3])
rnd = random.Random(int(sys.argv[4]) if len(sys.argv) > 4 else 1)
strata = {}
for k, (v, stones) in sorted(std.items()):
    if k in rh: strata.setdefault((v, rh[k]), []).append((k, stones))
for s in sorted(strata, reverse=True):
    pick = strata[s] if len(strata[s]) <= per else rnd.sample(strata[s], per)
    print(f'# stratum std={s[0]} rh={s[1]}: {len(strata[s])} positions, {len(pick)} sampled', file=sys.stderr)
    for k, stones in pick:
        print(k, stones, f'{s[0]}{s[1]}')
