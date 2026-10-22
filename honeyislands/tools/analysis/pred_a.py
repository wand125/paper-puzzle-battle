"""Prediction A: do winning moves determine more black cells than other moves?

For every position up to depth 2 (all children solved), compare the increase
of determined-black cells, dD = |D(child)| - |D(parent)|, between winning
moves (child lost for the opponent) and losing moves. One row per move edge.
Usage: python3 tools/analysis/pred_a.py results/book results/book-pass
"""
import collections, glob, statistics, sys

def load(base):
    value, dm = {}, {}
    for f in (glob.glob(f'{base}/res-*.txt') + glob.glob(f'{base}/depth3-results.txt')):
        for l in open(f):
            p = l.split()
            if len(p) >= 2: value[p[0]] = int(p[1])
    for l in open(f'{base}/positions.txt'):
        d, k, n, st, m, xm = l.split()
        dm[k] = int(m)
    edges, depth = [], {}
    for l in open(f'{base}/edges.txt'):
        d, pk, ck, c = l.split()
        edges.append((int(d) - 1, pk, ck, int(c)))
        depth[ck] = int(d); depth.setdefault(pk, int(d) - 1)
    kids = collections.defaultdict(list)
    for d, pk, ck, c in edges: kids[pk].append(ck)
    for d in (2, 1, 0):
        for k in [k for k, dd in depth.items() if dd == d]:
            vs = [value.get(c) for c in kids[k]]
            if any(v == 0 for v in vs): value[k] = 1
            elif vs and all(v == 1 for v in vs): value[k] = 0
    root = '0:0' if any(':' in k for k in dm) else '0'
    dm.setdefault(root, 0)
    return value, dm, edges

def mask(k): return int(k.split(':')[0])

for base in sys.argv[1:]:
    value, dm, edges = load(base)
    print(f'== {base}')
    for ply in (0, 1, 2):
        rows = []  # (dD, wins, is_pass, parent_wins)
        for d, pk, ck, c in edges:
            if d != ply or value.get(ck) is None: continue
            dD = bin(mask(ck)).count('1') - bin(mask(pk)).count('1')
            is_pass = mask(ck) == mask(pk)
            rows.append((dD, value[ck] == 0, is_pass, value.get(pk) == 1))
        # only positions where the mover has a choice between winning and losing moves matter
        win_pos = [r for r in rows if r[3]]
        w = [r[0] for r in win_pos if r[1] and not r[2]]
        l = [r[0] for r in win_pos if not r[1] and not r[2]]
        passes = [r for r in win_pos if r[2]]
        hist = collections.defaultdict(lambda: [0, 0])
        for r in rows:
            if r[2]: continue
            hist[r[0]][0 if r[1] else 1] += 1
        def m(x): return f'{statistics.mean(x):.2f}' if x else '-'
        print(f' ply {ply + 1}: moves in winning positions: winning {len(w)} (mean dD {m(w)}), losing {len(l)} (mean dD {m(l)}); pass moves {len(passes)} (winning {sum(r[1] for r in passes)})')
        print('   dD: win-rate of the move (all positions)  ' + '  '.join(f'{k}:{v[0]}/{v[0]+v[1]}' for k, v in sorted(hist.items())))
