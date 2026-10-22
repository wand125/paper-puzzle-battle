"""Depth-3 positions that prove the values of every depth-1 position (hence the
root and the value of each first move), choosing the cheapest children by the
recorded solve time. Writes verifier jobs "key stones".
Usage: python3 tools/analysis/proofset.py results/book out.txt [all]
"""
import collections, glob, sys

base, out = sys.argv[1], sys.argv[2]
res = {}
for f in (glob.glob(f'{base}/res-*.txt') + glob.glob(f'{base}/depth3-results.txt')):
    for l in open(f):
        p = l.split()
        if len(p) >= 7: res[p[0]] = (int(p[1]), float(p[4]))
pos = {}
for l in open(f'{base}/positions.txt'):
    d, k, n, st = l.split()[:4]
    pos[k] = (int(d), st)
kids, depth = collections.defaultdict(set), {}
for l in open(f'{base}/edges.txt'):
    d, pk, ck, _ = l.split()
    kids[pk].add(ck); depth[ck] = int(d); depth.setdefault(pk, int(d) - 1)
value = {k: v for k, (v, _) in res.items()}
for d in (2, 1, 0):
    for k in [k for k, dd in depth.items() if dd == d]:
        vs = [value[c] for c in kids[k]]
        value[k] = 1 if 0 in vs else 0
cost = lambda k: res[k][1]
need = set()
def prove(k):
    """set of depth-3 positions proving value[k]"""
    if depth[k] == 3:
        return {k}
    if value[k] == 1:   # one losing child suffices
        return min((prove(c) for c in kids[k] if value[c] == 0), key=lambda s: sum(map(cost, s)))
    s = set()
    for c in kids[k]:
        s |= prove(c)
    return s
if len(sys.argv) > 3 and sys.argv[3] == 'all':
    need = {k for k, d in depth.items() if d == 3}
else:
    for k in [k for k, d in depth.items() if d == 1]:
        need |= prove(k)
with open(out, 'w') as g:
    for k in sorted(need, key=cost):
        g.write(f'{k} {pos[k][1]}\n')
print(f'{base}: {len(need)} depth-3 positions, recorded solve time {sum(map(cost, need)) / 3.6e6:.2f} CPU-hours '
      f'(all 3,122: {sum(v[1] for v in res.values()) / 3.6e6:.2f}); wins {sum(value[k] for k in need)}')
